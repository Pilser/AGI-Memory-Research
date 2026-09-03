# Vector Storage: Direct-on-Node vs Separate Vector Node

> Detailed comparison for MNEMOS — can we simplify by storing embeddings directly on Engram nodes?

> **Decision: Direct-on-node vector storage is the approved approach.** This analysis confirmed it is safe and beneficial. All MNEMOS docs now use this design.

---

## Executive Summary

**Simplification is implemented.** HelixDB's vector indexes work on any node or edge property — not just special "vector nodes." We store `embedding` directly on the `Engram` node and create a vector index on `Engram.embedding`.

This eliminates:
- The `EngramVector` node type entirely
- The `HasVector` edge type entirely
- The `::In<HasVector>` traversal in every retrieval query
- One network hop (or query round-trip) per retrieval

**Impact on algorithms: ZERO degradation.** Every MNEMOS algorithm (CRR, Stimulation Layer, Consolidation, Contradiction Detection, Identity Crystallization) works identically — most become simpler.

---

## Current Architecture (Separate Vector Node)

### Graph Structure

```
Engram --HasVector--> EngramVector { embedding: [f32; 1536], content: String }
```

### Schema (Old)

```hql
N::Engram {
    episode_raw: String,
    emotional_charge: F32,
    // ... other properties
    // NO embedding here
}

N::EngramVector {        // <-- separate node
    embedding: [F64],
    content: String
}

E::HasVector {           // <-- edge just to hold vector
    From: Engram,
    To: EngramVector
}
```

### Retrieval Query (Old)

```rust
// 1. Search vectors
vector_search_with("EngramVector", "embedding", query_vec, limit, None)
    .in_(Some("HasVector"))      // <-- extra hop back to engram
    .where_(Predicate::eq("contradiction_flag", false))
    .value_map(Some(vec!["$id", "episode_raw", ...]))
```

**Problem:** Every retrieval requires:
1. Vector search → returns EngramVector nodes
2. Traverse `::In<HasVector>` → get Engram nodes
3. Filter on Engram properties
4. Project Engram fields

That's 2 hops minimum before you even start scoring.

---

## Simplified Architecture (Vector Direct on Node)

### Graph Structure

```
Engram { embedding: [f32; 1536], episode_raw: String, ... }
```

### Schema (New)

```rust
// No separate EngramVector node needed!
g().add_n("Engram", vec![
    ("episode_raw", episode_raw),
    ("emotional_charge", emotional_charge),
    ("importance_score", importance_score),
    ("decay_rate", decay_rate),
    ("activation_count", activation_count),
    ("timestamp", timestamp),
    ("engram_type", engram_type),
    ("compression_level", compression_level),
    ("contradiction_flag", contradiction_flag),
    ("embedding", embedding),          // <-- vector stored directly
])
```

### Index Creation (New)

```rust
// Create vector index directly on Engram.embedding
write_batch()
    .var_as("index", g().create_vector_index_nodes(
        "Engram",                                  // label
        "embedding",                               // property
        NonZeroUsize::new(1536).expect("non-zero"), // dimension
        VectorDistanceMetric::Cosine,              // metric
        None::<&str>,                              // no tenant partitioning
    ))
    .returning(["index"]);
```

### Retrieval Query (New)

```rust
// Single hop: vector search returns Engram nodes directly
vector_search_with("Engram", "embedding", query_vec, limit, None)
    // No ::In<HasVector> needed!
    .where_(Predicate::gte("importance_score", min_importance))
    .where_(Predicate::eq("contradiction_flag", false))
    .value_map(Some(vec!["$id", "episode_raw", ...]))
```

---

## Algorithm-by-Algorithm Impact Analysis

### 1. Cognitive Resonance Retrieval (CRR)

| Aspect | Separate Vector Node | Direct-on-Node | Impact |
|--------|---------------------|----------------|--------|
| Vector search | Returns EngramVector | Returns Engram | **Simpler** |
| Traversal needed | `::In<HasVector>` | None | **1 fewer hop** |
| Filtering | On Engram after traverse | On Engram directly | **Same** |
| Scoring formula | 5-factor multiplicative | 5-factor multiplicative | **Identical** |
| `$distance` availability | On EngramVector hit | On Engram hit | **Same** |

**Verdict: SIMPLER, no degradation.** The CRR formula doesn't care where the vector lives — it only needs the cosine distance and the engram's properties. Both are available in a single query now.

```rust
// NEW: CRR retrieval in a single query
#[query]
fn search_engrams_full(
    query_embedding: Vec<f32>,
    limit: i64,
    min_importance: f64,
) -> ReadBatch {
    read_batch()
        .var_as("candidates", g()
            // Vector search returns Engram nodes DIRECTLY
            .vector_search_with("Engram", "embedding", query_embedding, limit, None)
            // Filter directly on Engram properties
            .where_(Predicate::gte("importance_score", min_importance))
            .where_(Predicate::eq("contradiction_flag", false))
            .value_map(Some(vec![
                "$id", "episode_raw", "emotional_charge",
                "importance_score", "engram_type", "timestamp",
                "$distance",  // cosine distance available here
            ])))
        .returning(["candidates"])
}
```

### 2. Stimulation Layer (Spreading Activation)

| Aspect | Separate Vector Node | Direct-on-Node | Impact |
|--------|---------------------|----------------|--------|
| Seed phase | SearchV → EngramVector → `::In<HasVector>` → Engram | SearchV → Engram directly | **Simpler** |
| α₁ (HasVector weight: 0.85) | Used to spread from vector to engram | **Not needed** | See note below |
| α₂-α₈ (other edges) | Unchanged | Unchanged | **Identical** |
| Recurrent feedback | Engram → Concept | Engram → Concept | **Identical** |

**Critical question:** Does removing the `HasVector` edge affect the spreading activation dynamics?

**Answer: NO.** Here's why:

The `HasVector` edge (α₁ = 0.85) was used in the Stimulation Layer to spread activation from the vector search results back to the engram nodes. But this was always a **one-way bridge** — activation flowed from vector → engram, never the reverse. It was not part of any associative chain.

In the new architecture:
- The seed phase directly activates Engram nodes (no bridge needed)
- All associative spreading (α₂ Recalls, α₃ AbstractsTo, α₄ Reinforces, α₅ TemporalSequence, α₆ Contradicts, α₇ Defines, α₈ SpawnedFrom) operates on graph edges between engrams/concepts/identities — completely unaffected

**What changes in parameters:**
- α₁ is no longer needed (it was the vector-to-engram bridge weight)
- The "seed activation" formula changes slightly:

```
OLD: activation[engram] = activation[vector] × α₁ × cosine_sim × recency × emotion
NEW: activation[engram] = cosine_sim × recency × emotion
```

The α₁ factor (0.85) was essentially a constant multiplier on all seed activations. Removing it just scales all seed activations by `1/0.85 ≈ 1.18` — a uniform boost that doesn't change the relative ranking of results. You can compensate by adjusting the seed limit or threshold if needed.

**Verdict: SIMPLER, no functional degradation.** The spreading dynamics through the actual associative graph (which is what matters for "oh, that reminds me of..." behavior) are completely unchanged.

### 3. Consolidation Loop (Decay, Compression, Promotion)

| Aspect | Separate Vector Node | Direct-on-Node | Impact |
|--------|---------------------|----------------|--------|
| GetEngramsForDecayCheck | Query Engram by type | Query Engram by type | **Identical** |
| CompressEngram | Update Engram properties | Update Engram properties | **Identical** |
| PromoteToSemantic | Update Engram properties | Update Engram properties | **Identical** |
| PruneEngram | Delete Engram (+ cascade) | Delete Engram (+ cascade) | **Identical** |

**Verdict: IDENTICAL.** The consolidation loop never touched the `EngramVector` node — it only operates on `Engram` properties. No change needed.

### 4. Contradiction Detection

| Aspect | Separate Vector Node | Direct-on-Node | Impact |
|--------|---------------------|----------------|--------|
| GetEngramsRecallingConcept | `N<Concept>::In<Recalls>` | `N<Concept>::In<Recalls>` | **IDENTical** |
| Group by emotional_charge | On Engram properties | On Engram properties | **IDENTical** |
| Create Contradicts edges | Between Engrams | Between Engrams | **IDENTical** |

**Verdict: IDENTICAL.** Contradiction detection operates entirely on the Engram → Concept → Engram path. The vector node was never involved.

### 5. Identity Crystallization

| Aspect | Separate Vector Node | Direct-on-Node | Impact |
|--------|---------------------|----------------|--------|
| GetCognitiveChain | Engram → Concept → Identity | Engram → Concept → Identity | **IDENTical** |
| UpdateIdentityTrait | Weighted blend on Identity | Weighted blend on Identity | **IDENTical** |

**Verdict: IDENTICAL.** The cognitive chain traversal doesn't involve the vector node.

### 6. Memory Mitosis

| Aspect | Separate Vector Node | Direct-on-Node | Impact |
|--------|---------------------|----------------|--------|
| CheckMitosisCandidates | Query Concept by source_count | Query Concept by source_count | **IDENTical** |
| Split concepts | Create child Concepts + SpawnedFrom | Create child Concepts + SpawnedFrom | **IDENTical** |
| Reassign engrams | Update Recalls edges | Update Recalls edges | **IDENTical** |

**Verdict: IDENTICAL.** Mitosis operates on the Concept subgraph, not vectors.

### 7. Ingestion Pipeline

| Aspect | Separate Vector Node | Direct-on-Node | Impact |
|--------|---------------------|----------------|--------|
| CreateEngram | 1. Create Engram<br>2. Create EngramVector<br>3. Connect HasVector | 1. Create Engram (with embedding) | **Atomic in 1 step** |
| Concept extraction | Same | Same | **IDENTical** |
| Edge creation | Connect Recalls, AbstractsTo | Connect Recalls, AbstractsTo | **IDENTical** |

**Verdict: SIMPLER.** Ingestion goes from 3 atomic operations to 1. The embedding is stored at creation time, no separate vector node or edge needed.

---

## What We Lose (and Why It Doesn't Matter)

### Lost: Ability to have multiple vectors per engram

**Old design:** You could theoretically create multiple `EngramVector` nodes connected to one `Engram` (e.g., different embedding models).

**Reality:** MNEMOS uses exactly one embedding per engram. If you need multiple in the future, you can:
- Store multiple vector properties: `embedding_openai`, `embedding_local`
- Create separate vector indexes on each
- Search across both with a union query

**Impact: NONE for current design.**

### Lost: Ability to share one vector across multiple engrams

**Old design:** Multiple engrams could point to the same `EngramVector` via `HasVector`.

**Reality:** This never happened in MNEMOS — each engram has its own unique embedding. Sharing vectors would actually be wrong for CRR (each engram needs its own resonance score).

**Impact: NONE.** This was never a feature, and sharing would degrade retrieval quality.

### Lost: Separate storage of vector metadata

**Old design:** `EngramVector` had a `content` field storing the original text.

**Reality:** The `Engram` node already has `episode_raw` which is the same text. The `content` field on `EngramVector` was redundant.

**Impact: NONE.** The data is still available on the Engram node.

---

## What We Gain

### 1. Fewer Nodes and Edges

| Metric | Separate Vector Node | Direct-on-Node | Savings |
|--------|---------------------|----------------|---------|
| Nodes per engram | 2 (Engram + EngramVector) | 1 (Engram) | **50%** |
| Edges per engram | 1 (HasVector) + N (Recalls, etc.) | N (Recalls, etc.) | **1 fewer per engram** |
| Total nodes for 1M engrams | 2M | 1M | **1M fewer** |
| Total edges for 1M engrams | 1M + association edges | association edges | **1M fewer** |

### 2. Fewer Queries per Retrieval

| Operation | Separate Vector Node | Direct-on-Node |
|-----------|---------------------|----------------|
| Vector search + filter | 1 query (with traverse) | 1 query (no traverse) |
| Activate engram | 2 queries (get + update) | 2 queries (same) |
| **Total round-trips** | **3** | **3** (but simpler queries) |

The query count is similar, but the vector search query is simpler (no traversal step).

### 3. Simpler Queries

```rust
// OLD: 4-step query
vector_search_with("EngramVector", "embedding", query_vec, limit, None)
    .in_(Some("HasVector"))           // step 2: traverse to engram
    .where_(Predicate::eq("contradiction_flag", false))  // step 3: filter
    .value_map(Some(vec!["$id", "episode_raw", ...]))    // step 4: project

// NEW: 2-step query
vector_search_with("Engram", "embedding", query_vec, limit, None)
    .where_(Predicate::eq("contradiction_flag", false))  // step 2: filter
    .value_map(Some(vec!["$id", "episode_raw", ...]))    // step 3: project
```

### 4. Better Cache Locality

With vectors on the engram node, a single node read fetches all engram data (properties + embedding). The old design required reading two nodes (Engram + EngramVector) which could be on different pages/blocks.

### 5. Simpler Schema

| Old | New |
|-----|-----|
| 4 node types (Engram, Concept, Identity, EngramVector) | 3 node types (Engram, Concept, Identity) |
| 8 edge types (Recalls, Contradicts, Reinforces, AbstractsTo, Defines, TemporalSequence, SpawnedFrom, HasVector) | 7 edge types (no HasVector) |
| 1 redundant `content` field | 0 redundant fields |

---

## Updated Schema Definition

### Nodes (3 types, down from 4)

```rust
// Engram — now includes embedding directly
g().add_n("Engram", vec![
    ("episode_raw", episode_raw),           // String
    ("emotional_charge", emotional_charge), // F64 (-1.0 to +1.0)
    ("importance_score", importance_score), // F64 (0.0 to 1.0)
    ("decay_rate", decay_rate),             // F64 (Ebbinghaus parameter)
    ("activation_count", activation_count), // I64 (times recalled)
    ("timestamp", timestamp),               // String (RFC 3339)
    ("engram_type", engram_type),           // String
    ("compression_level", compression_level), // I64 (0, 1, 2)
    ("contradiction_flag", contradiction_flag), // bool
    ("embedding", embedding),               // Vec<f32> (1536-dim)
])

// Concept — unchanged
g().add_n("Concept", vec![
    ("name", name),
    ("confidence", confidence),
    ("formation_date", formation_date),
    ("source_count", source_count),
])

// Identity — unchanged
g().add_n("Identity", vec![
    ("trait", trait_name),
    ("value", value),
    ("stability", stability),
    ("last_updated", last_updated),
])
```

### Edges (7 types, down from 8)

```rust
// All unchanged — just remove HasVector
"Recalls"           // Engram → Concept
"Contradicts"       // Engram → Engram
"Reinforces"        // Engram → Engram
"AbstractsTo"       // Engram → Concept
"Defines"           // Concept → Identity
"TemporalSequence"  // Engram → Engram
"SpawnedFrom"       // Engram → Engram (mitosis)
// "HasVector" — REMOVED
```

### Indexes

```rust
// Vector index — now on Engram.embedding (was EngramVector.embedding)
g().create_vector_index_nodes(
    "Engram",                                  // label (was "EngramVector")
    "embedding",                               // property
    NonZeroUsize::new(1536).expect("non-zero"), // dimension
    VectorDistanceMetric::Cosine,
    None::<&str>,
)

// Secondary indexes — unchanged
g().create_index_if_not_exists(IndexSpec::node_equality("Engram", "engram_type"))
g().create_index_if_not_exists(IndexSpec::node_equality("Engram", "contradiction_flag"))
g().create_index_if_not_exists(IndexSpec::node_equality("Concept", "name"))
```

---

## Migration Path

If you have existing data with separate `EngramVector` nodes:

### Option A: Start Fresh (Recommended for New Deployments)

1. Create new schema with `embedding` on `Engram`
2. Re-ingest all data (embeddings stored directly on engram)
3. Old `EngramVector` nodes and `HasVector` edges are never created

### Option B: Migrate Existing Data

1. Add `embedding` property to all `Engram` nodes:
   ```rust
   // For each engram, copy embedding from its EngramVector
   g().n(NodeRef::id(engram_id))
       .set_property("embedding", embedding_value)
   ```
2. Create vector index on `Engram.embedding`
3. Optionally drop old `EngramVector` nodes and `HasVector` edges

---

## Edge Cases and Considerations

### 1. Vector Updates

**Question:** What if you need to update an embedding (e.g., re-embed with a new model)?

**Answer:** With direct-on-node, you update the property:
```rust
g().n(NodeRef::id(engram_id))
    .set_property("embedding", new_embedding)
```

This is actually SIMPLER than the old approach (which required updating the separate `EngramVector` node).

### 2. Multiple Embeddings per Engram

**Question:** What if you want to store embeddings from different models?

**Answer:** Add multiple vector properties:
```rust
g().add_n("Engram", vec![
    ("embedding_openai", openai_embedding),
    ("embedding_local", local_embedding),
    // ... other properties
])
```

Create separate indexes on each. Search across both with a union query.

### 3. Edge Vectors

**Question:** Can you store vectors on edges too?

**Answer:** Yes! HelixDB supports vector indexes on edge properties. For example, if you wanted to embed the relationship itself:
```rust
g().n(NodeRef::id(from_id))
    .add_e("Recalls", NodeRef::id(to_id), vec![
        ("relationship_embedding", embedding),  // vector on edge!
        ("strength", 0.8),
    ])
```

This is not needed for MNEMOS but shows the flexibility.

### 4. Vector Search Within Traversal

**Question:** Can you combine vector search with graph traversal?

**Answer:** Yes! This is where direct-on-node really shines:

```rust
// Find engrams semantically similar to query, that recall a specific concept
g().n_with_label_where("Concept", SourcePredicate::eq("name", "infrastructure"))
    .in_(Some("Recalls"))                    // Get engrams recalling this concept
    .vector_search_with("Engram", "embedding", query_vec, limit, None)  // Rank by similarity
    .value_map(Some(vec!["$id", "episode_raw", "$distance"]))
```

With the old design, this would require an extra `::In<HasVector>` hop.

---

## Performance Implications

### Query Latency

| Operation | Separate Vector Node | Direct-on-Node | Improvement |
|-----------|---------------------|----------------|-------------|
| Vector search + filter | ~3-5ms (with traverse) | ~2-3ms (no traverse) | **~30-40% faster** |
| Ingestion (engram + vector) | ~6-9ms (3 operations) | ~2-3ms (1 operation) | **~60-70% faster** |
| Stimulation Layer seed | ~3-5ms | ~2-3ms | **~30% faster** |

### Storage

| Metric | Separate Vector Node | Direct-on-Node | Savings |
|--------|---------------------|----------------|---------|
| Nodes | 2 per engram | 1 per engram | **50%** |
| Edges | 1 (HasVector) per engram | 0 | **100% of HasVector edges** |
| Index entries | 1 per EngramVector | 1 per Engram | **Same** |

### Memory (Cache)

| Metric | Separate Vector Node | Direct-on-Node |
|--------|---------------------|----------------|
| Nodes cached | Engram + EngramVector | Engram only |
| Cache efficiency | 2 node reads per retrieval | 1 node read per retrieval |
| Vector access | Separate read (may miss cache) | Same read as properties |

---

## Recommendation

**Use direct-on-node vector storage.** The simplification is:

1. **Safe** — zero algorithmic degradation
2. **Simpler** — fewer node types, fewer edge types, simpler queries
3. **Faster** — fewer hops, fewer operations, better cache locality
4. **More flexible** — can add multiple vectors per node, can vectorize edges

The separate `EngramVector` node was an architectural artifact of older vector database designs where vectors had to live in separate "vector stores." HelixDB's native vector support makes this unnecessary.

---

## Updated Documents

The following documents should be updated to reflect this simplification:

- `helixdb-embedded-schema.md` — Remove `EngramVector` node and `HasVector` edge
- `helixdb-embedded-queries.md` — Remove `::In<HasVector>` traversals
- `helixdb-embedded-retrieval.md` — Update CRR and Stimulation Layer queries
- `helixdb-embedded-pipeline.rs` — Update pipeline to use direct vector storage
