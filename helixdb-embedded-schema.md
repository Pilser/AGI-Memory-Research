# HelixDB Embedded Schema — MNEMOS Reference

> How the old `schema.hx` HelixQL schema maps to the new Rust DSL dynamic schema creation

---

## Overview

The old approach defined schema in a separate `schema.hx` file using HelixQL DDL. The new SDK defines schema **dynamically in Rust** using the DSL — `add_n()` for nodes, `add_e()` for edges, and `create_vector_index_nodes()` / `create_index_if_not_exists()` for indexes.

There is no separate schema file. Schema is created by executing write batches that add the first node/edge of each type, and by explicitly creating indexes.

---

## Old vs New Comparison

### Old: Static HelixQL DDL (`schema.hx`)

```hql
N::Engram {
    episode_raw:       String,
    emotional_charge:  F32,
    importance_score:  F32,
    decay_rate:        F32,
    activation_count:  U32,
    timestamp:         Date,
    engram_type:       String,
    compression_level: U8,
    contradiction_flag: Boolean
}

E::Recalls {
    From: Engram,
    To: Concept
}
```

### New: Dynamic Rust DSL

```rust
use helix_db::dsl::prelude::*;

// Create an Engram node (schema is implicit from the first write)
let request = write_batch()
    .var_as("engram", g().add_n("Engram", vec![
        ("episode_raw", "User talked about..."),
        ("emotional_charge", 0.7),
        ("importance_score", 0.8),
        ("decay_rate", 0.05),
        ("activation_count", 0_i64),
        ("timestamp", "2026-09-03T12:00:00Z"),
        ("engram_type", "episodic"),
        ("compression_level", 0_i64),
        ("contradiction_flag", false),
    ]))
    .returning(["engram"]);

// Create a Recalls edge between engram and concept
let edge_request = write_batch()
    .var_as("edge", g()
        .n(NodeRef::var("engram_id"))
        .add_e("Recalls", NodeRef::var("concept_id"), vec![]))
    .returning(["edge"]);
```

**Key difference:** Schema is not declared separately — it emerges from the data you write. The first `add_n("Engram", ...)` with a set of properties defines what an Engram looks like.

---

## Full Schema in Rust DSL

### Node Types

#### Engram (Core Memory Unit — includes embedding directly)

```rust
g().add_n("Engram", vec![
    ("episode_raw", episode_raw),           // String — raw memory text
    ("emotional_charge", emotional_charge), // F64 — -1.0 to +1.0
    ("importance_score", importance_score), // F64 — 0.0 to 1.0
    ("decay_rate", decay_rate),             // F64 — Ebbinghaus parameter
    ("activation_count", activation_count), // I64 — times recalled (use I64 for U32)
    ("timestamp", timestamp),               // String (RFC 3339 date-time)
    ("engram_type", engram_type),           // String — "episodic"|"semantic"|"procedural"
    ("compression_level", compression_level), // I64 — 0=raw, 1=compressed, 2=crystallized
    ("contradiction_flag", contradiction_flag), // bool
    ("embedding", embedding),               // Vec<f32> — 1536-dimension embedding (DIRECTLY ON NODE)
])
```

**Key change:** The embedding is stored directly on the Engram node. No separate `EngramVector` node or `HasVector` edge needed.

#### Concept

```rust
g().add_n("Concept", vec![
    ("name", name),                         // String
    ("confidence", confidence),             // F64 — 0.0 to 1.0
    ("formation_date", formation_date),     // String (RFC 3339 date-time)
    ("source_count", source_count),         // I64 — number of linked engrams
])
```

#### Concept (Optional Enhancement: Centroid Embedding)

> **This is an OPTIONAL enhancement that improves functionality without degrading anything.**
> Store a centroid embedding on Concept nodes to enable concept-level vector search and faster mitosis decisions.

```rust
g().add_n("Concept", vec![
    ("name", name),                         // String
    ("confidence", confidence),             // F64 — 0.0 to 1.0
    ("formation_date", formation_date),     // String (RFC 3339 date-time)
    ("source_count", source_count),         // I64 — number of linked engrams
    ("embedding", centroid_embedding),      // Vec<f32> — OPTIONAL centroid of all recalling engrams
])
```

**How it works:**
- After mitosis creates child concepts, compute the centroid (average) of all engrams' embeddings that recall each child
- Store that centroid on the Concept node
- Update the centroid whenever engrams are added/removed from a concept

**Benefits (no degradation):**
- **CRR:** Can search concepts first as a pre-filter, then drill down to engrams
- **Mitosis:** Faster decisions — compare concept centroids instead of re-clustering all engrams
- **Stimulation Layer:** Can seed activation from concepts AND engrams simultaneously

**Cost:** Extra computation to maintain centroids (update on engram assignment changes)

#### Identity

```rust
g().add_n("Identity", vec![
    ("trait", trait_name),                  // String
    ("value", value),                       // F32 — trait strength
    ("stability", stability),               // F32 — resistance to change
    ("last_updated", last_updated),         // String (RFC 3339 date-time)
])
```

**Note on types:** The Rust DSL uses `i64` for integer properties (there's no `u32` or `u64` wire type — use `i64` and cast). Float properties accept `f64` values. Boolean accepts `bool`. String accepts `&str` or `String`. Vector properties use `Vec<f32>`.

---

### Edge Types

#### Recalls (Engram → Concept)

```rust
g().n(NodeRef::var("engram_id")).add_e("Recalls", NodeRef::var("concept_id"), vec![])
```

#### Recalls (Optional Enhancement: Edge Embedding)

> **This is an OPTIONAL enhancement that improves the Stimulation Layer without degrading anything.**
> Store a relationship embedding on each Recalls edge to enable dynamic edge weights.

```rust
g().n(NodeRef::var("engram_id")).add_e("Recalls", NodeRef::var("concept_id"), vec![
    ("relationship_embedding", edge_embedding),  // Vec<f32> — how this engram relates to this concept
    ("strength", 0.85),                          // F64 — optional strength indicator
])
```

**How it works:**
- When creating a Recalls edge, compute a "relationship embedding" that captures HOW this engram relates to this concept
- This is different from the engram's own embedding (which captures WHAT the engram is about)
- Example: An engram about "Uganda ICT hub" might relate to concept "infrastructure sovereignty" through a POLITICAL angle or TECHNICAL angle

**Benefits (no degradation):**
- **Stimulation Layer:** Enables dynamic edge weights — stronger relationships transmit more activation (more biologically plausible)
- **Memory Mitosis:** Can cluster by relationship embeddings for more nuanced concept formation
- **CRR:** Optional combined scoring (engram similarity × relationship strength)

**Cost:** Extra storage (one embedding per edge) and computation (compute at edge creation)

#### Contradicts (Engram → Engram)

```rust
g().n(NodeRef::var("engram_a_id")).add_e("Contradicts", NodeRef::var("engram_b_id"), vec![])
```

#### Reinforces (Engram → Engram)

```rust
g().n(NodeRef::var("engram_a_id")).add_e("Reinforces", NodeRef::var("engram_b_id"), vec![])
```

#### AbstractsTo (Engram → Concept)

```rust
g().n(NodeRef::var("engram_id")).add_e("AbstractsTo", NodeRef::var("concept_id"), vec![])
```

#### Defines (Concept → Identity)

```rust
g().n(NodeRef::var("concept_id")).add_e("Defines", NodeRef::var("identity_id"), vec![])
```

#### TemporalSequence (Engram → Engram)

```rust
g().n(NodeRef::var("earlier_id")).add_e("TemporalSequence", NodeRef::var("later_id"), vec![])
```

#### SpawnedFrom (Engram → Engram, for mitosis)

```rust
g().n(NodeRef::var("parent_id")).add_e("SpawnedFrom", NodeRef::var("child_id"), vec![])
```

---

## Indexes

Indexes are NOT created automatically. You must explicitly create them before they can be used for search.

### Vector Index (for semantic search)

Required for `vector_search_with` operations on embeddings:

```rust
use helix_db::dsl::prelude::*;
use std::num::NonZeroUsize;

// Create vector index directly on Engram.embedding
let request = write_batch()
    .var_as("index", g().create_vector_index_nodes(
        "Engram",                                     // label (vector stored directly on Engram)
        "embedding",                                  // property
        NonZeroUsize::new(1536).expect("non-zero"),   // dimension
        VectorDistanceMetric::Cosine,                 // distance metric
        None::<&str>,                                 // no tenant partitioning
    ))
    .returning(["index"]);

let response = client.query(request).send().await?;
```

**Distance metrics available:**
- `VectorDistanceMetric::Cosine` — best for semantic similarity (most common for MNEMOS)
- `VectorDistanceMetric::Euclidean` — L2 distance
- `VectorDistanceMetric::Manhattan` — L1 distance

**Important constraints:**
- Dimension must match exactly — stored vectors and query vectors must both be 1536-dim
- Creation returns before backfill finishes — poll until index is active
- Malformed source vectors block activation
- For cosine: no component magnitude limit
- For euclidean/manhattan: each component must be ≤ `sqrt(f32::MAX / (8 × dim))` or `f32::MAX / (4 × dim)`

### Secondary Indexes (for exact lookup)

For filtering by exact property values:

```rust
// Index on Engram.engram_type (for filtering episodic/semantic/procedural)
let request = write_batch()
    .var_as("index", g().create_index_if_not_exists(
        IndexSpec::node_equality("Engram", "engram_type")
    ))
    .returning(["index"]);

// Index on Engram.contradiction_flag (for excluding contradictions)
let request = write_batch()
    .var_as("index", g().create_index_if_not_exists(
        IndexSpec::node_equality("Engram", "contradiction_flag")
    ))
    .returning(["index"]);

// Index on Concept.name (for looking up concepts by name)
let request = write_batch()
    .var_as("index", g().create_index_if_not_exists(
        IndexSpec::node_equality("Concept", "name")
    ))
    .returning(["index"]);
```

### Text Indexes (for BM25 search)

For full-text search on text content:

```rust
let request = write_batch()
    .var_as("index", g().create_text_index_nodes(
        "Engram",       // label
        "episode_raw",  // property
        "standard",     // analyzer
    ))
    .returning(["index"]);
```

### Optional: Vector Index on Concept Centroid

> **Optional enhancement** — enables concept-level vector search.
> Only needed if you add the `embedding` property to Concept nodes.

```rust
// Create vector index on Concept.embedding (for concept-level search)
let request = write_batch()
    .var_as("index", g().create_vector_index_nodes(
        "Concept",                                    // label
        "embedding",                                  // property (centroid embedding)
        NonZeroUsize::new(1536).expect("non-zero"),   // dimension
        VectorDistanceMetric::Cosine,                 // distance metric
        None::<&str>,                                 // no tenant partitioning
    ))
    .returning(["index"]);
```

**Benefits:**
- Search concepts directly by semantic similarity
- Use as pre-filter for CRR (search concepts → get their engrams → score individually)
- Faster mitosis decisions (compare concept centroids)

---

## Index Lifecycle

Creating an index is an **asynchronous durable operation**:

1. **Create** — returns a DDL receipt immediately
2. **Backfill** — existing data is scanned and indexed in the background
3. **Validate** — all vectors are checked for correct dimension and magnitude
4. **Activate** — index becomes visible and usable for queries

**You must poll until the index is active** before depending on indexed performance:

```rust
// After creating index, poll until active
// (exact polling mechanism depends on SDK response structure)
let receipt = client.query(create_index_request).send().await?;
// Poll receipt.status until "active"
```

---

## Schema Creation Order

For MNEMOS, create in this order:

1. **Vector index** on `Engram.embedding` — needed before any vector search
2. **Secondary indexes** on frequently filtered properties:
   - `Engram.engram_type`
   - `Engram.contradiction_flag`
   - `Concept.name`
3. **Text index** on `Engram.episode_raw` (optional, for BM25 search)
4. **Vector index on `Concept.embedding`** (optional — only if using concept centroid enhancement)

Indexes can be created at any time, but vector search won't work until the vector index is active.

---

## Type Mapping Reference

| Old HelixQL Type | Rust DSL Type | Notes |
|-----------------|---------------|-------|
| `String` | `&str` or `String` | Direct mapping |
| `F32` | `f64` | DSL accepts f64, stores as needed |
| `I64` | `i64` | Direct mapping |
| `U32` | `i64` | No unsigned wire type — use i64 |
| `U8` | `i64` | No unsigned wire type — use i64 |
| `U64` | `i64` | No unsigned wire type — use i64 |
| `Boolean` | `bool` | Direct mapping |
| `Date` | `&str` (RFC 3339) | e.g., "2026-09-03T12:00:00Z" |
| `[F64]` (vector) | `Vec<f32>` | For embedding properties |

---

## Migration Notes

### What Changed

| Old | New |
|-----|-----|
| `N::Engram { ... }` in `schema.hx` | `g().add_n("Engram", vec![...])` in Rust |
| `E::Recalls { From: Engram, To: Concept }` | `g().n(...).add_e("Recalls", ...)` in Rust |
| Schema defined once, deployed separately | Schema emerges from writes + explicit index creation |
| Indexes defined in schema file | Indexes created via `create_vector_index_nodes()` / `create_index_if_not_exists()` |

### What Stayed the Same

- Node labels: `Engram`, `Concept`, `Identity` (no more `EngramVector`)
- Edge labels: `Recalls`, `Contradicts`, `Reinforces`, `AbstractsTo`, `Defines`, `TemporalSequence`, `SpawnedFrom` (no more `HasVector`)
- Property names and semantics
- Vector search concept (cosine similarity on embeddings)

### What's New (Direct Vector Storage)

- **Embedding on Engram:** The `embedding` property is stored directly on the `Engram` node
- **No separate vector node:** Eliminates `EngramVector` node type and `HasVector` edge type
- **Simpler retrieval:** Vector search returns `Engram` nodes directly — no `::In<HasVector>` traversal needed
- **Fewer entities:** 50% fewer nodes, 1 fewer edge per engram

### What's New

- **Schema is code** — no separate DDL files, schema is created by running Rust functions
- **Indexes are explicit** — must be created and activated before use
- **Type coercion** — unsigned integers use `i64` wire type
- **Dynamic properties** — can add properties to nodes without schema migration

---

## Optional Enhancements Summary

> These enhancements ADD functionality WITHOUT degrading any existing algorithm.
> Each is independent — adopt none, some, or all.

| Enhancement | What it adds | Algorithms improved | Degradation? |
|-------------|--------------|---------------------|--------------|
| **Concept Centroid Embedding** | `embedding` property on Concept | CRR (pre-filter), Mitosis (faster decisions), Stimulation Layer (new seed option) | **None** |
| **Edge Embedding on Recalls** | `relationship_embedding` property on Recalls edges | Stimulation Layer (dynamic weights), Mitosis (relationship-based clustering) | **None** |
| **Concept Vector Index** | Vector search on Concept.embedding | CRR (concept-level search), Mitosis (centroid comparison) | **None** |

**Implementation order (if adopting):**
1. Concept centroid embedding (biggest impact, lowest cost)
2. Edge embedding on Recalls (for dynamic Stimulation Layer weights)
3. Concept vector index (only if using concept-level search)

---

## Related Documentation

- [Integration Guide](./helixdb-embedded-integration.md) — SDK setup, client init, storage backends
- [Query Reference](./helixdb-embedded-queries.md) — all `#[query]` functions
- [Retrieval Layer](./helixdb-embedded-retrieval.md) — CRR + Stimulation Layer
- [Full Pipeline](./helixdb-embedded-pipeline.rs) — complete reference implementation
- [FutureMemoryAGI.md](./FutureMemoryAGI.md) — system overview and architecture
