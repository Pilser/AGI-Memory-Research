# Consolidation Queries — Ebbinghaus Decay & Memory Lifecycle

These queries are called by the **consolidation loop** (`Pipeline.consolidate()`), which runs as a background process between user sessions — the AI's "sleep" cycle. They govern the full lifecycle of engrams: decay, compression, pruning, promotion, mitosis, and source count maintenance.

> **Key simplification:** Since embeddings are stored directly on `Engram` nodes, pruning an engram automatically removes its embedding too — no orphan vector cleanup needed.

---

## 1. GetEngramsForDecayCheck

### What It Does
Returns all engrams of a given type that are still at `compression_level = 0` (raw, uncompressed). The application code then computes the Ebbinghaus retention formula in Rust and decides whether to compress, prune, or promote each engram.

### Parameters
| Param | Type | Purpose |
|---|---|---|
| `engram_type` | String | Typically "episodic" (episodic memories decay, semantic/crystallized don't) |

### Returns
```json
{
    "engrams": [
        { "$id": 42, "decay_rate": 0.05, "activation_count": 3, "timestamp": "2026-04-01T...", "importance_score": 0.8 }
    ]
}
```

### Retention Formula (computed in Rust, NOT in HelixQL)
```rust
let days_elapsed = (now - timestamp).num_days() as f64;
let retention = (-decay_rate * days_elapsed / ((activation_count + 1) as f64).sqrt()).exp();
```

### Used By
- `Pipeline.consolidate()` — the main decay loop

### Query Implementation
```rust
use helix_db::dsl::prelude::*;

#[query]
fn get_engrams_for_decay_check(engram_type: String) -> ReadBatch {
    read_batch()
        .var_as("engrams", g()
            .n_with_label_where("Engram", SourcePredicate::eq("engram_type", engram_type))
            .where_(Predicate::eq("compression_level", 0_i64))
            .value_map(Some(vec![
                "$id", "decay_rate", "activation_count", "timestamp", "importance_score",
            ])))
        .returning(["engrams"])
}
```

---

## 2. CompressEngram

### What It Does
Compresses an engram: sets `compression_level = 1` and clears the `episode_raw` text (replaces with "COMPRESSED"). The engram node, its embedding, and all edges are preserved — only the raw content is gone. This frees storage while keeping the structural graph connections.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | u64 |

### Triggered When
```
retention < 0.15 AND importance_score < 0.3
```

### Used By
- Consolidation loop — for engrams that are fading but still have structural value

### Query Implementation
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

---

## 3. PromoteToSemantic

### What It Does
Promotes an engram to permanent semantic memory: sets `engram_type = "semantic"` and `compression_level = 2` (crystallized). The engram's raw text, embedding, and all edges are preserved. It will no longer be scanned for decay in future consolidation cycles.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | u64 |

### Triggered When
```
retention > 0.8 AND activation_count > 20
```

Meaning: the engram has been recalled many times and is resisting decay — it's become long-term knowledge.

### Used By
- Consolidation loop — for engrams that have proven themselves durable

### Query Implementation
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

---

## 4. PruneEngram

### What It Does
**Deletes** an engram node from the graph. Since the embedding is stored directly on the `Engram` node, deleting the node automatically removes the embedding too — no orphan vector cleanup needed.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | u64 |

### Triggered When
```
retention < 0.05 AND importance_score < 0.1
```

Meaning: the engram is mostly forgotten AND was never important.

### Important
HelixDB cascade-deletes all edges connected to the engram (`Recalls`, `AbstractsTo`, `Contradicts`, `Reinforces`, `TemporalSequence`, `SpawnedFrom`). The embedding is deleted along with the node — **no separate vector cleanup step required**.

### Used By
- Consolidation loop — for engrams that have been forgotten

### Query Implementation
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

### Typical Query Patterns
```rust
// Prune a forgotten engram — embedding is automatically removed
let request = prune_engram(engram_id);
client.query(request).send().await?;
// No orphan vector cleanup needed!
```

---

## 5. ActivateEngram

### What It Does
Increments the `activation_count` on an engram by 1. Called every time an engram is recalled during retrieval. Higher `activation_count` slows future decay — frequently accessed memories resist forgetting.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | u64 |
| `current_activation_count` | i64 |

### Used By
- `Pipeline.retrieve_memories()` — after returning search results, bumps activation_count on top-ranked engrams
- Not called during consolidation — only during retrieval

### Effect on Decay
```
retention = exp(-decay_rate * days / sqrt(activation_count + 1))

activation_count = 0  → sqrt(1) = 1.0   → normal decay
activation_count = 3  → sqrt(4) = 2.0   → decay is halved
activation_count = 15 → sqrt(16) = 4.0  → decay is quartered
activation_count = 99 → sqrt(100) = 10.0 → decay is 10x slower
```

This is the **spacing effect** — the more you recall something, the slower you forget it.

### Query Implementation
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

**Note:** The DSL requires you to read the current value first (via `get_engram`), then call `activate_engram()` with it. This is typically:
```rust
let engram = get_engram(engram_id);
let current_count = engram.activation_count;
let request = activate_engram(engram_id, current_count);
client.query(request).send().await?;
```

---

## 6. CheckMitosisCandidates

### What It Does
Returns all concepts whose `source_count` meets or exceeds a threshold. These are candidates for memory mitosis — the concept may be overloaded and ready to split into specialized children.

### Parameters
| Param | Type | Purpose |
|---|---|---|
| `min_engram_count` | i64 | Minimum source_count to trigger mitosis (e.g., 10) |

### Returns
```json
{
    "concepts": [
        { "$id": 12, "name": "Uganda ICT", "confidence": 0.85, "source_count": 15 }
    ]
}
```

### Used By
- `Pipeline.consolidate()` — mitosis check step

### What Happens Next (in Rust)
For each concept returned:
1. `GetEngramsRecallingConcept(concept_id)` → engram data (embeddings on nodes)
2. Cluster embeddings (HDBSCAN / K-means)
3. For each cluster:
   - Generate child concept name (LLM or centroid-based)
   - `CreateConcept(name, confidence)` → child_id
   - `ConnectSpawn(parent_id, child_id)`
   - Reassign engrams: drop old Recalls/AbstractsTo, create new ones to child
4. `UpdateConceptSourceCount(parent_id)` — recalculate after reassignment

### Query Implementation
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

---

## 7. UpdateConceptSourceCount

### What It Does
Recalculates the `source_count` on a concept by counting the number of incoming `Recalls` edges. Keeps the `source_count` field in sync with actual graph structure. Should be called after any operation that changes `Recalls` edges (ingestion, mitosis reassignment, pruning).

### Parameters
| Param | Type |
|---|---|
| `concept_id` | u64 |
| `new_count` | i64 |

### Used By
- `Pipeline.ingest_memory()` — after adding Recalls edges for a new engram
- Mitosis Splitter — after reassigning engrams to child concepts
- Any time Recalls edges change

### Query Implementation
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

## Consolidation Loop Execution Order

```rust
Pipeline.consolidate():

  // 1. DECAY
  let engrams = get_engrams_for_decay_check("episodic").await?;
  for engram in engrams {
      let retention = compute_retention(&engram);
      if retention < 0.05 && engram.importance_score < 0.1 {
          prune_engram(engram.id).await?;
          // No orphan vector cleanup needed — embedding is on the node!
      } else if retention < 0.15 && engram.importance_score < 0.3 {
          compress_engram(engram.id).await?;
      } else if retention > 0.8 && engram.activation_count > 20 {
          promote_to_semantic(engram.id).await?;
      }
  }

  // 2. MITOSIS
  let concepts = check_mitosis_candidates(10).await?;
  for concept in concepts {
      let engrams = get_engrams_recalling_concept(concept.id).await?;
      // engram.embedding available directly on each engram node
      let clusters = hdbscan_cluster(&engrams);
      for cluster in clusters {
          let child_id = create_concept(&cluster.name, cluster.confidence).await?;
          connect_spawn(concept.id, child_id).await?;
          // Reassign Recalls/AbstractsTo edges
          update_concept_source_count(child_id, cluster.size).await?;
      }
      update_concept_source_count(concept.id, remaining_count).await?;
  }

  // 3. CONTRADICTION SCAN
  for concept in concepts {
      let engrams = get_engrams_recalling_concept(concept.id).await?;
      // Group by emotional_charge sign, detect cross-group contradictions
  }

  // 4. IDENTITY CRYSTALLIZATION
  for concept in concepts {
      if concept.confidence > 0.7 && concept.source_count > 20 {
          // Create or update Identity via weighted blending
      }
  }
```
