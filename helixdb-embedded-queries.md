# HelixDB Embedded Queries — MNEMOS Reference

> Every query from the old `queries.hx` converted to the new `#[query]` Rust DSL functions

---

## Overview

The old approach defined queries in a separate `queries.hx` file using HelixQL, then called them by name from Rust using `client.query("QueryName", &json!({...}))`. The new approach defines queries as **Rust functions** with the `#[query]` macro — typed arguments become parameters, the function name becomes `query_name`.

This document shows every old query alongside its new DSL equivalent.

---

## Pattern: The `#[query]` Macro

### Old: Separate HelixQL + Rust call

```hql
// queries.hx
QUERY CreateEngram(episode_raw: String, ...) =>
    engram <- AddN<Engram>({ episode_raw: episode_raw, ... })
    RETURN engram
```

```rust
// pipeline.rs (old)
let result = client.query("CreateEngram", &json!({
    "episode_raw": "User discussed...",
    "emotional_charge": 0.7,
    ...
})).await?;
```

### New: Single `#[query]` function

```rust
// queries.rs (new)
use helix_db::dsl::prelude::*;

#[query]
fn create_engram(
    episode_raw: String,
    emotional_charge: f64,
    importance_score: f64,
    decay_rate: f64,
    activation_count: i64,
    engram_type: String,
    compression_level: i64,
    contradiction_flag: bool,
) -> WriteBatch {
    write_batch()
        .var_as("engram", g().add_n("Engram", vec![
            ("episode_raw", episode_raw),
            ("emotional_charge", emotional_charge),
            ("importance_score", importance_score),
            ("decay_rate", decay_rate),
            ("activation_count", activation_count),
            ("timestamp", chrono::Utc::now().to_rfc3339()),
            ("engram_type", engram_type),
            ("compression_level", compression_level),
            ("contradiction_flag", contradiction_flag),
        ]))
        .returning(["engram"])
}

// Usage:
let request = create_engram(
    "User discussed...".to_string(),
    .7, .8, .05, 0, "episodic".to_string(), 0, false
);
let response = client.query(request).send().await?;
```

**Key behaviors of `#[query]`:**
- Typed function arguments → request parameters (auto-serialized)
- Function name → `query_name` (e.g., `create_engram` becomes `"create_engram"`)
- Return type: `ReadBatch` for reads, `WriteBatch` for writes
- Parameters appear in `parameters` and `parameter_types` fields of the request envelope

---

## Create Operations

### CreateEngram

```rust
#[query]
fn create_engram(
    episode_raw: String,
    emotional_charge: f64,
    importance_score: f64,
    decay_rate: f64,
    activation_count: i64,
    engram_type: String,
    compression_level: i64,
    contradiction_flag: bool,
) -> WriteBatch {
    write_batch()
        .var_as("engram", g().add_n("Engram", vec![
            ("episode_raw", episode_raw),
            ("emotional_charge", emotional_charge),
            ("importance_score", importance_score),
            ("decay_rate", decay_rate),
            ("activation_count", activation_count),
            ("timestamp", chrono::Utc::now().to_rfc3339()),
            ("engram_type", engram_type),
            ("compression_level", compression_level),
            ("contradiction_flag", contradiction_flag),
        ]))
        .returning(["engram"])
}
```

### CreateConcept

```rust
#[query]
fn create_concept(
    name: String,
    confidence: f64,
) -> WriteBatch {
    write_batch()
        .var_as("concept", g().add_n("Concept", vec![
            ("name", name),
            ("confidence", confidence),
            ("formation_date", chrono::Utc::now().to_rfc3339()),
            ("source_count", 1_i64),
        ]))
        .returning(["concept"])
}
```

### CreateIdentity

```rust
#[query]
fn create_identity(
    trait_name: String,
    value: f64,
    stability: f64,
) -> WriteBatch {
    write_batch()
        .var_as("identity", g().add_n("Identity", vec![
            ("trait", trait_name),
            ("value", value),
            ("stability", stability),
            ("last_updated", chrono::Utc::now().to_rfc3339()),
        ]))
        .returning(["identity"])
}
```

### CreateEngramVectorText (REMOVED — no longer needed)

**This query is eliminated.** With direct-on-node vector storage, you don't create separate `EngramVector` nodes. The embedding is stored directly on the `Engram` node via `create_full_engram()`.

### CreateEngramVector (REMOVED — no longer needed)

**This query is eliminated.** Same reason — embeddings live on the Engram node directly.

---

## Edge Operations

### ConnectEngramVector (REMOVED — no longer needed)

**This query is eliminated.** With direct-on-node vector storage, there's no `EngramVector` node to connect. The embedding is already on the `Engram` node.

### ConnectRecalls

```rust
#[query]
fn connect_recalls(
    engram_id: u64,
    concept_id: u64,
) -> WriteBatch {
    write_batch()
        .var_as("edge", g()
            .n(NodeRef::id(engram_id))
            .add_e("Recalls", NodeRef::id(concept_id), vec![]))
        .returning(["edge"])
}
```

### ConnectRecalls (Optional Enhancement: With Edge Embedding)

> **Optional enhancement** — stores relationship embedding on the edge for dynamic Stimulation Layer weights.

```rust
#[query]
fn connect_recalls_with_embedding(
    engram_id: u64,
    concept_id: u64,
    relationship_embedding: Vec<f32>,
    strength: f64,
) -> WriteBatch {
    write_batch()
        .var_as("edge", g()
            .n(NodeRef::id(engram_id))
            .add_e("Recalls", NodeRef::id(concept_id), vec![
                ("relationship_embedding", relationship_embedding),
                ("strength", strength),
            ]))
        .returning(["edge"])
}
```

**When to use:**
- Use `connect_recalls` for simple connections (no degradation)
- Use `connect_recalls_with_embedding` when you want dynamic edge weights in the Stimulation Layer

### ConnectContradiction

```rust
#[query]
fn connect_contradiction(
    engram_a_id: u64,
    engram_b_id: u64,
) -> WriteBatch {
    write_batch()
        .var_as("edge", g()
            .n(NodeRef::id(engram_a_id))
            .add_e("Contradicts", NodeRef::id(engram_b_id), vec![]))
        .returning(["edge"])
}
```

### ConnectReinforcement

```rust
#[query]
fn connect_reinforcement(
    engram_a_id: u64,
    engram_b_id: u64,
) -> WriteBatch {
    write_batch()
        .var_as("edge", g()
            .n(NodeRef::id(engram_a_id))
            .add_e("Reinforces", NodeRef::id(engram_b_id), vec![]))
        .returning(["edge"])
}
```

### ConnectAbstraction

```rust
#[query]
fn connect_abstraction(
    engram_id: u64,
    concept_id: u64,
) -> WriteBatch {
    write_batch()
        .var_as("edge", g()
            .n(NodeRef::id(engram_id))
            .add_e("AbstractsTo", NodeRef::id(concept_id), vec![]))
        .returning(["edge"])
}
```

### ConnectDefinition

```rust
#[query]
fn connect_definition(
    concept_id: u64,
    identity_id: u64,
) -> WriteBatch {
    write_batch()
        .var_as("edge", g()
            .n(NodeRef::id(concept_id))
            .add_e("Defines", NodeRef::id(identity_id), vec![]))
        .returning(["edge"])
}
```

### ConnectTemporal

```rust
#[query]
fn connect_temporal(
    earlier_id: u64,
    later_id: u64,
) -> WriteBatch {
    write_batch()
        .var_as("edge", g()
            .n(NodeRef::id(earlier_id))
            .add_e("TemporalSequence", NodeRef::id(later_id), vec![]))
        .returning(["edge"])
}
```

### ConnectSpawn

```rust
#[query]
fn connect_spawn(
    parent_id: u64,
    child_id: u64,
) -> WriteBatch {
    write_batch()
        .var_as("edge", g()
            .n(NodeRef::id(parent_id))
            .add_e("SpawnedFrom", NodeRef::id(child_id), vec![]))
        .returning(["edge"])
}
```

---

## Atomic: CreateFullEngram

This is the key atomic operation — creates an engram, its vector, and connects them, all in one write batch transaction.

```rust
#[query]
fn create_full_engram(
    episode_raw: String,
    emotional_charge: f64,
    importance_score: f64,
    decay_rate: f64,
    engram_type: String,
    compression_level: i64,
    contradiction_flag: bool,
    embedding: Vec<f32>,
) -> WriteBatch {
    write_batch()
        // Step 1: Create the Engram node
        .var_as("engram", g().add_n("Engram", vec![
            ("episode_raw", episode_raw),
            ("emotional_charge", emotional_charge),
            ("importance_score", importance_score),
            ("decay_rate", decay_rate),
            ("activation_count", 0_i64),
            ("timestamp", chrono::Utc::now().to_rfc3339()),
            ("engram_type", engram_type),
            ("compression_level", compression_level),
            ("contradiction_flag", contradiction_flag),
        ]))
        // Step 2: Create the vector node
        .var_as("vector", g().add_n("EngramVector", vec![
            ("embedding", embedding),
            ("content", episode_raw),
        ]))
        // Step 3: Connect engram → vector with HasVector edge
        .var_as("has_vector", g()
            .n(NodeRef::var("engram"))
            .add_e("HasVector", NodeRef::var("vector"), vec![]))
        .returning(["engram", "vector"])
}
```

**Important:** The old approach made 3 separate HTTP calls. The new approach does it all in one atomic transaction — either all succeed or none do.

---

## Read Operations

### GetEngram

```rust
#[query]
fn get_engram(engram_id: u64) -> ReadBatch {
    read_batch()
        .var_as("engram", g()
            .n(NodeRef::id(engram_id))
            .value_map(Some(vec!["$id", "episode_raw", "emotional_charge", "importance_score", "decay_rate", "activation_count", "timestamp", "engram_type", "compression_level", "contradiction_flag"])))
        .returning(["engram"])
}
```

### GetEngramWithVector

```rust
#[query]
fn get_engram_with_vector(engram_id: u64) -> ReadBatch {
    read_batch()
        .var_as("result", g()
            .n(NodeRef::id(engram_id))
            .value_map(Some(vec!["$id", "episode_raw", "emotional_charge", "importance_score"])))
        .returning(["result"])
}
```

### GetEngramsByType

```rust
#[query]
fn get_engrams_by_type(engram_type: String) -> ReadBatch {
    read_batch()
        .var_as("engrams", g()
            .n_with_label_where("Engram", SourcePredicate::eq("engram_type", engram_type))
            .value_map(Some(vec!["$id", "episode_raw", "emotional_charge", "importance_score", "timestamp"])))
        .returning(["engrams"])
}
```

### GetConceptByName

```rust
#[query]
fn get_concept_by_name(name: String) -> ReadBatch {
    read_batch()
        .var_as("concept", g()
            .n_with_label_where("Concept", SourcePredicate::eq("name", name))
            .limit(1)
            .value_map(Some(vec!["$id", "name", "confidence", "source_count"])))
        .returning(["concept"])
}
```

### GetRecalledConcepts

```rust
#[query]
fn get_recalled_concepts(engram_id: u64) -> ReadBatch {
    read_batch()
        .var_as("concepts", g()
            .n(NodeRef::id(engram_id))
            .out(Some("Recalls"))
            .value_map(Some(vec!["$id", "name", "confidence"])))
        .returning(["concepts"])
}
```

### GetEngramsRecallingConcept

```rust
#[query]
fn get_engrams_recalling_concept(concept_id: u64) -> ReadBatch {
    read_batch()
        .var_as("engrams", g()
            .n(NodeRef::id(concept_id))
            .in_(Some("Recalls"))
            .value_map(Some(vec!["$id", "episode_raw", "emotional_charge"])))
        .returning(["engrams"])
}
```

### GetContradictions

```rust
#[query]
fn get_contradictions(engram_id: u64) -> ReadBatch {
    read_batch()
        .var_as("outgoing", g()
            .n(NodeRef::id(engram_id))
            .out(Some("Contradicts"))
            .value_map(Some(vec!["$id", "episode_raw", "emotional_charge"])))
        .var_as("incoming", g()
            .n(NodeRef::id(engram_id))
            .in_(Some("Contradicts"))
            .value_map(Some(vec!["$id", "episode_raw", "emotional_charge"])))
        .returning(["outgoing", "incoming"])
}
```

### GetIdentityAlignment

```rust
#[query]
fn get_identity_alignment(engram_id: u64) -> ReadBatch {
    read_batch()
        .var_as("identity_traits", g()
            .n(NodeRef::id(engram_id))
            .out(Some("Recalls"))
            .out(Some("Defines"))
            .value_map(Some(vec!["$id", "trait", "value", "stability"])))
        .returning(["identity_traits"])
}
```

### GetCognitiveChain

```rust
#[query]
fn get_cognitive_chain(engram_id: u64) -> ReadBatch {
    read_batch()
        .var_as("engram", g()
            .n(NodeRef::id(engram_id))
            .value_map(Some(vec!["$id", "episode_raw", "emotional_charge", "importance_score", "engram_type"])))
        .var_as("concepts", g()
            .n(NodeRef::id(engram_id))
            .out(Some("Recalls"))
            .value_map(Some(vec!["$id", "name", "confidence"])))
        .var_as("identities", g()
            .n(NodeRef::id(engram_id))
            .out(Some("Recalls"))
            .out(Some("Defines"))
            .value_map(Some(vec!["$id", "trait", "value", "stability"])))
        .returning(["engram", "concepts", "identities"])
}
```

---

## Vector Search Operations

### SearchEngramsByVector

```rust
#[query]
fn search_engrams_by_vector(
    query_vector: Vec<f32>,
    limit: i64,
) -> ReadBatch {
    read_batch()
        .var_as("results", g()
            // Vector search returns Engram nodes DIRECTLY (no ::In<HasVector> needed)
            .vector_search_with("Engram", "embedding", query_vector, limit, None)
            .value_map(Some(vec!["$id", "$distance", "episode_raw"])))
        .returning(["results"])
}
```

### SearchEngramsByText

```rust
#[query]
fn search_engrams_by_text(
    query_embedding: Vec<f32>,
    limit: i64,
) -> ReadBatch {
    read_batch()
        .var_as("results", g()
            // Vector search returns Engram nodes DIRECTLY
            .vector_search_with("Engram", "embedding", query_embedding, limit, None)
            .value_map(Some(vec!["$id", "$distance", "episode_raw"])))
        .returning(["results"])
}
```

### SearchEngramsFull

This is the main retrieval query — vector search + filter (no traversal needed).

```rust
#[query]
fn search_engrams_full(
    query_embedding: Vec<f32>,
    limit: i64,
    min_importance: f64,
) -> ReadBatch {
    read_batch()
        .var_as("candidates", g()
            // Step 1: Vector search returns Engram nodes DIRECTLY
            .vector_search_with("Engram", "embedding", query_embedding, limit, None)
            // Step 2: Filter by importance and contradiction flag (no traversal needed!)
            .where_(Predicate::gte("importance_score", min_importance))
            .where_(Predicate::eq("contradiction_flag", false))
            // Step 3: Project needed fields
            .value_map(Some(vec!["$id", "episode_raw", "emotional_charge", "importance_score", "engram_type", "timestamp"])))
        .returning(["candidates"])
}
```

**Key simplification:** No `::In<HasVector>` traversal needed. Vector search returns `Engram` nodes directly.

---

## Decay & Consolidation Operations

### GetEngramsForDecayCheck

```rust
#[query]
fn get_engrams_for_decay_check(engram_type: String) -> ReadBatch {
    read_batch()
        .var_as("engrams", g()
            .n_with_label_where("Engram", SourcePredicate::eq("engram_type", engram_type))
            .where_(Predicate::eq("compression_level", 0_i64))
            .value_map(Some(vec!["$id", "decay_rate", "activation_count", "timestamp", "importance_score"])))
        .returning(["engrams"])
}
```

### CompressEngram

```rust
#[query]
fn compress_engram(engram_id: u64) -> WriteBatch {
    write_batch()
        .var_as("compressed", g()
            .n(NodeRef::id(engram_id))
            .set_property("compression_level", 1_i64)
            .set_property("episode_raw", "COMPRESSED"))
        .returning(["compressed"])
}
```

### PromoteToSemantic

```rust
#[query]
fn promote_to_semantic(engram_id: u64) -> WriteBatch {
    write_batch()
        .var_as("promoted", g()
            .n(NodeRef::id(engram_id))
            .set_property("engram_type", "semantic")
            .set_property("compression_level", 2_i64))
        .returning(["promoted"])
}
```

### PruneEngram

```rust
#[query]
fn prune_engram(engram_id: u64) -> WriteBatch {
    write_batch()
        .var_as("pruned", g()
            .n(NodeRef::id(engram_id))
            .drop())
        .returning(["pruned"])
}
```

### ActivateEngram

```rust
#[query]
fn activate_engram(
    engram_id: u64,
    current_activation_count: i64,
) -> WriteBatch {
    write_batch()
        .var_as("updated", g()
            .n(NodeRef::id(engram_id))
            .set_property("activation_count", current_activation_count + 1))
        .returning(["updated"])
}
```

**Note:** The old approach used `ADD(engram::{activation_count}, 1)` which read and incremented in one step. The new DSL requires you to read the current value first, then set the incremented value. This is typically done by first calling `get_engram()` to get the current count, then calling `activate_engram()` with it.

### CheckMitosisCandidates

```rust
#[query]
fn check_mitosis_candidates(min_engram_count: i64) -> ReadBatch {
    read_batch()
        .var_as("concepts", g()
            .n_with_label("Concept")
            .where_(Predicate::gte("source_count", min_engram_count))
            .value_map(Some(vec!["$id", "name", "confidence", "source_count"])))
        .returning(["concepts"])
}
```

### UpdateConceptSourceCount

```rust
#[query]
fn update_concept_source_count(
    concept_id: u64,
    new_count: i64,
) -> WriteBatch {
    write_batch()
        .var_as("updated", g()
            .n(NodeRef::id(concept_id))
            .set_property("source_count", new_count))
        .returning(["updated"])
}
```

---

## Identity Operations

### GetAllIdentity

```rust
#[query]
fn get_all_identity() -> ReadBatch {
    read_batch()
        .var_as("traits", g()
            .n_with_label("Identity")
            .value_map(Some(vec!["$id", "trait", "value", "stability", "last_updated"])))
        .returning(["traits"])
}
```

### UpdateIdentityTrait

```rust
#[query]
fn update_identity_trait(
    identity_id: u64,
    new_value: f64,
    stability: f64,
    current_value: f64,
) -> WriteBatch {
    // Weighted update: new = old * stability + evidence * (1 - stability)
    let blended = current_value * stability + new_value * (1.0 - stability);

    write_batch()
        .var_as("updated", g()
            .n(NodeRef::id(identity_id))
            .set_property("value", blended)
            .set_property("last_updated", chrono::Utc::now().to_rfc3339()))
        .returning(["updated"])
}
```

---

## Optional Enhancement: Concept Centroid Embedding

### UpdateConceptCentroid (Optional Enhancement)

> **Optional enhancement** — computes and stores centroid embedding on a concept.
> Call this after mitosis creates child concepts or when engrams are reassigned.

```rust
#[query]
fn update_concept_centroid(
    concept_id: u64,
    centroid_embedding: Vec<f32>,
) -> WriteBatch {
    write_batch()
        .var_as("updated", g()
            .n(NodeRef::id(concept_id))
            .set_property("embedding", centroid_embedding))
        .returning(["updated"])
}
```

**When to call:**
- After mitosis creates child concepts (compute centroid from assigned engrams)
- After reassigning engrams to a different concept (recompute both old and new concept centroids)
- After ingestion links a new engram to an existing concept

**Computing the centroid (client-side):**
```rust
async fn compute_and_update_centroid(
    &self,
    concept_id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Get all engrams recalling this concept
    let engrams_request = get_engrams_recalling_concept(concept_id);
    let engrams_response = self.client.query(engrams_request).send().await?;
    let engrams: Vec<EngramData> = serde_json::from_value(
        engrams_response["engrams"].clone()
    )?;

    // 2. Compute centroid (average of all engram embeddings)
    let centroid = compute_centroid(&engrams);

    // 3. Store centroid on the concept
    let update_request = update_concept_centroid(concept_id, centroid);
    self.client.query(update_request).send().await?;

    Ok(())
}

fn compute_centroid(engrams: &[EngramData]) -> Vec<f32> {
    let dim = 1536;
    let mut centroid = vec![0.0f32; dim];
    for engram in engrams {
        for (i, val) in engram.embedding.iter().enumerate() {
            centroid[i] += val;
        }
    }
    let n = engrams.len() as f32;
    for val in &mut centroid {
        *val /= n;
    }
    centroid
}
```

---

## Contradiction Detection

### DetectContradictionCandidates

```rust
#[query]
fn detect_contradiction_candidates(concept_id: u64) -> ReadBatch {
    read_batch()
        .var_as("engrams", g()
            .n(NodeRef::id(concept_id))
            .in_(Some("Recalls"))
            .value_map(Some(vec!["$id", "episode_raw", "emotional_charge", "importance_score"])))
        .returning(["engrams"])
}
```

### GetContradictedEngrams

```rust
#[query]
fn get_contradicted_engrams() -> ReadBatch {
    read_batch()
        .var_as("engrams", g()
            .n_with_label_where("Engram", SourcePredicate::eq("contradiction_flag", true))
            .where_(Predicate::exists("_::Out<Contradicts>"))  // has outgoing contradiction edges
            .value_map(Some(vec!["$id", "episode_raw", "engram_type"])))
        .returning(["engrams"])
}
```

---

## Temporal Operations

### GetTimeline

```rust
#[query]
fn get_timeline(
    limit: i64,
    offset: i64,
) -> ReadBatch {
    read_batch()
        .var_as("engrams", g()
            .n_with_label("Engram")
            // Note: ORDER BY support in Rust DSL may be limited
            // Alternative: fetch all and sort client-side
            .limit(limit + offset)
            .value_map(Some(vec!["$id", "episode_raw", "engram_type", "emotional_charge", "timestamp"])))
        .returning(["engrams"])
}
```

**Note on ordering:** The old approach used `ORDER<Desc>(_::{timestamp})` and `RANGE(offset, limit)`. The new DSL's Rust SDK may not expose direct ORDER BY / RANGE operations. The workaround is to fetch with a limit and handle pagination client-side, or use the JSON AST directly for advanced operations.

### GetTemporalChain

```rust
#[query]
fn get_temporal_chain(engram_id: u64) -> ReadBatch {
    read_batch()
        .var_as("next", g()
            .n(NodeRef::id(engram_id))
            .out(Some("TemporalSequence"))
            .value_map(Some(vec!["$id", "episode_raw", "timestamp"])))
        .var_as("prev", g()
            .n(NodeRef::id(engram_id))
            .in_(Some("TemporalSequence"))
            .value_map(Some(vec!["$id", "episode_raw", "timestamp"])))
        .returning(["next", "prev"])
}
```

---

## Aggregation & Statistics

### CountEngramsByType

```rust
#[query]
fn count_engrams_by_type() -> ReadBatch {
    read_batch()
        .var_as("counts", g()
            .n_with_label("Engram")
            .count())
        .returning(["counts"])
}
```

### GetMemoryStats

```rust
#[query]
fn get_memory_stats() -> ReadBatch {
    read_batch()
        .var_as("total", g()
            .n_with_label("Engram")
            .count())
        .var_as("contradictions", g()
            .n_with_label_where("Engram", SourcePredicate::eq("contradiction_flag", true))
            .count())
        .var_as("concepts", g()
            .n_with_label("Concept")
            .count())
        .var_as("identities", g()
            .n_with_label("Identity")
            .count())
        .returning(["total", "contradictions", "concepts", "identities"])
}
```

---

## Transaction Behavior

All entries in a single `write_batch()` commit or roll back together. This is critical for MNEMOS:

```rust
// This is ONE transaction — all succeed or none do
let request = write_batch()
    .var_as("engram", g().add_n("Engram", vec![...]))      // entry 1
    .var_as("vector", g().add_n("EngramVector", vec![...]))  // entry 2
    .var_as("edge", g()                                      // entry 3
        .n(NodeRef::var("engram"))
        .add_e("HasVector", NodeRef::var("vector"), vec![]))
    .returning(["engram", "vector"]);
```

**Reads inside a write batch can refer to earlier mutations** through named variables. For example, entry 3 can reference the result of entry 1 via `NodeRef::var("engram")`.

---

## Returning Semantics

`returning()` controls what appears in the response — NOT what executes:

```rust
// All three entries execute, but only "engram" appears in the response
write_batch()
    .var_as("engram", g().add_n("Engram", vec![...]))       // executes
    .var_as("vector", g().add_n("EngramVector", vec![...]))  // executes
    .var_as("edge", g()                                       // executes
        .n(NodeRef::var("engram"))
        .add_e("HasVector", NodeRef::var("vector"), vec![]))
    .returning(["engram"])                                    // only this in response
```

This is useful for write-only steps that don't need to return data (like creating edges).

---

## Migration Notes

### What Changed

| Old Pattern | New Pattern |
|-------------|-------------|
| `QUERY Name(params) => ...` in `queries.hx` | `#[query] fn name(params) -> BatchType { ... }` in Rust |
| `client.query("Name", &json!({...}))` | `client.query(request).send().await` |
| `AddN<Label>({ prop: value })` | `g().add_n("Label", vec![("prop", value)])` |
| `AddE<Label>::From(id)::To(id)` | `g().n(NodeRef::id(from)).add_e("Label", NodeRef::id(to), vec![])` |
| `N<Label>(id)::Out<Edge>` | `g().n(NodeRef::id(id)).out(Some("Edge"))` |
| `N<Label>::WHERE(pred)` | `g().n_with_label_where("Label", SourcePredicate::eq(...))` |
| `UpsertN({ prop: value })` | `g().n(NodeRef::id(id)).set_property("prop", value)` |
| `DROP N<Label>(id)` | `g().n(NodeRef::id(id)).drop()` |
| `SearchV<EngramVector>(vec, limit)::In<HasVector>` | `g().vector_search_with("Engram", "embedding", vec, limit, None)` |
| Separate `EngramVector` node + `HasVector` edge | **Eliminated** — embedding stored directly on `Engram` |

### What Stayed the Same

- The graph data model (nodes, edges, properties)
- Transaction scope (one transaction per query invocation)
- The request envelope structure (`request_type`, `query_name`, `query`)
- Vector search, full-text search, secondary indexes

### What's New

- **Atomic multi-step operations** — create engram + vector + edge in one call
- **Type-safe parameters** — `#[query]` macro generates parameter types
- **Compile-time safety** — can't use `add_n` in a read batch (compile error)
- **No separate query files** — queries are Rust functions

### Known Limitations

| Old Feature | New Status | Workaround |
|-------------|-----------|------------|
| `UpsertN` (read-modify-write in one step) | Not directly available | Read current value, then `set_property` |
| `ORDER BY` | Limited/no DSL support | Sort client-side after fetch |
| `GROUP BY` | Limited/no DSL support | Aggregate client-side |
| `RANGE(offset, limit)` | Limited/no DSL support | Use `limit()` and skip client-side |
| `Embed()` function (server-side embedding) | Not available in embedded | Compute embedding client-side, pass as parameter |
| `NOW` keyword | Not available | Use `chrono::Utc::now().to_rfc3339()` client-side |
| `EXISTS(path)` | Limited support | Use `.where_()` with appropriate predicate |
| `COUNT` as subquery (e.g., `concept::In<Recalls>::COUNT`) | Not directly available | Fetch connected nodes and count client-side |

---

## Related Documentation

- [Integration Guide](./helixdb-embedded-integration.md) — SDK setup, client init, storage backends
- [Schema Definition](./helixdb-embedded-schema.md) — nodes, edges, indexes
- [Retrieval Layer](./helixdb-embedded-retrieval.md) — CRR + Stimulation Layer
- [Full Pipeline](./helixdb-embedded-pipeline.rs) — complete reference implementation
- [FutureMemoryAGI.md](./FutureMemoryAGI.md) — system overview and architecture
