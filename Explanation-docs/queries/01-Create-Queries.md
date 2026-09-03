# Create Queries — Writing Data to DB

These queries are called at **ingestion time** and during **consolidation** to write nodes, vectors, and edges into the graph.

> **Key simplification:** Embeddings are stored directly on `Engram` nodes (`embedding: Vec<f32>`). There is no `EngramVector` node type and no `HasVector` edge type.

---

## 1. CreateEngram

### What It Does
Creates a bare `Engram` node with all fields manually specified — including the embedding vector directly on the node. Used when you need full control over every field — but usually you want `CreateFullEngram` instead, which also handles timestamp automatically.

### Parameters
| Param | Type | Source |
|---|---|---|
| `episode_raw` | String | User/system input (raw memory text) |
| `emotional_charge` | f64 | Emotional Tagger (ML) |
| `importance_score` | f64 | Importance Scorer (ML) |
| `decay_rate` | f64 | System (default 0.05) |
| `activation_count` | i64 | System (0 at creation) |
| `engram_type` | String | System ("episodic", "semantic", "procedural") |
| `compression_level` | i64 | System (0 = raw) |
| `contradiction_flag` | bool | System (false at creation) |
| `embedding` | Vec<f32> | Embedding model (1536 dims, computed client-side) |

### Returns
The created Engram node (with auto-assigned ID and `timestamp`).

### Used By
- Manual engram creation (testing, debugging)
- Pipeline: not directly — use `CreateFullEngram` instead

### Query Implementation
```rust
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
    embedding: Vec<f32>,
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
            ("embedding", embedding),
        ]))
        .returning(["engram"])
}
```

### Typical Query Patterns
```rust
// Create a fully-specified engram with embedding
let request = create_engram(
    "User discussed Uganda ICT infrastructure".to_string(),
    0.7, 0.8, 0.05, 0, "episodic".to_string(), 0, false,
    embedding_vector,  // Vec<f32> from your embedding model
);
let response = client.query(request).send().await?;
```

---

## 2. CreateConcept

### What It Does
Creates a `Concept` node. Called by the Concept Extractor for each concept extracted from an engram.

### Parameters
| Param | Type | Source |
|---|---|---|
| `name` | String | Concept Extractor (ML) |
| `confidence` | f64 | Concept Extractor (confidence in this concept) |

### Returns
The created Concept node (with auto-assigned `formation_date`, `source_count: 1`).

### Used By
- `Pipeline.ingest_memory()` — one call per extracted concept
- Mitosis Splitter — when creating child concepts

### Important
Before calling this, always check if the concept already exists via `GetConceptByName`. Calling this for an existing concept creates a duplicate.

### Query Implementation
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

---

## 3. CreateIdentity

### What It Does
Creates an `Identity` node — a stable personality trait.

### Parameters
| Param | Type | Source |
|---|---|---|
| `trait_name` | String | System / consolidation loop |
| `value` | f64 | Derived from concept confidence (0.0–1.0) |
| `stability` | f64 | System (0.5 for new traits, increases with time) |

### Returns
The created Identity node (with `last_updated`).

### Used By
- Identity crystallization (consolidation loop)
- Manual identity seeding

### Query Implementation
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

---

## 4. ConnectRecalls

### What It Does
Creates a `Recalls` edge: Engram → Concept. "This memory mentions/recalls this concept."

### Parameters
| Param | Type | Source |
|---|---|---|
| `engram_id` | u64 | The engram linking to the concept |
| `concept_id` | u64 | The concept being recalled |

### Used By
- `Pipeline.ingest_memory()` — one call per extracted concept
- Mitosis — reassigning engrams to child concepts

### Query Implementation
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

### Optional Enhancement: With Edge Embedding
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

---

## 5. ConnectContradiction

### What It Does
Creates a `Contradicts` edge between two engrams that contradict each other.

### Parameters
| Param | Type | Source |
|---|---|---|
| `engram_a_id` | u64 | First engram |
| `engram_b_id` | u64 | Second engram (contradicts the first) |

### Used By
- Contradiction Detector (ML) — when opposite emotional charges found on same concept
- Consolidation loop — batch contradiction scanning

### Query Implementation
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

---

## 6. ConnectReinforcement

### What It Does
Creates a `Reinforces` edge between two engrams that support/confirm each other.

### Parameters
| Param | Type | Source |
|---|---|---|
| `engram_a_id` | u64 | Reinforcing engram |
| `engram_b_id` | u64 | Reinforced engram |

### Used By
- Consolidation loop — when multiple engrams about the same concept have the same emotional sign

### Query Implementation
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

---

## 7. ConnectAbstraction

### What It Does
Creates an `AbstractsTo` edge: Engram → Concept. "This memory feeds into/abstracts to this concept." Paired with every `Recalls` edge.

### Parameters
| Param | Type | Source |
|---|---|---|
| `engram_id` | u64 | The engram |
| `concept_id` | u64 | The abstracted concept |

### Used By
- `Pipeline.ingest_memory()` — paired with `ConnectRecalls`
- Mitosis — reassigning abstractions to child concepts

### Query Implementation
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

---

## 8. ConnectDefinition

### What It Does
Creates a `Defines` edge: Concept → Identity. "This knowledge defines this identity trait."

### Parameters
| Param | Type | Source |
|---|---|---|
| `concept_id` | u64 | The concept crystallizing into identity |
| `identity_id` | u64 | The identity trait being defined |

### Used By
- Identity crystallization (consolidation loop)
- When a concept's confidence + source_count crosses threshold

### Query Implementation
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

---

## 9. ConnectTemporal

### What It Does
Creates a `TemporalSequence` edge from an earlier engram to a later one, establishing chronological order.

### Parameters
| Param | Type | Source |
|---|---|---|
| `earlier_id` | u64 | The engram that happened first |
| `later_id` | u64 | The engram that happened after |

### Used By
- Ingestion — links new engram to the previous most-recent engram
- Consolidation — batch timeline ordering

### Query Implementation
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

---

## 10. ConnectSpawn

### What It Does
Creates a `SpawnedFrom` edge: Parent Concept → Child Concept. "This specialized concept was spawned from this general concept."

### Parameters
| Param | Type | Source |
|---|---|---|
| `parent_id` | u64 | The parent concept (general) |
| `child_id` | u64 | The child concept (specialized, from mitosis) |

### Used By
- Memory Mitosis Splitter — one call per child concept spawned

### Query Implementation
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

## 11. CreateFullEngram (PRIMARY INGESTION QUERY)

### What It Does
The **main ingestion query**. Creates an engram node with its embedding stored directly on the node — all in one atomic operation. No separate vector node or edge needed.

### Parameters
| Param | Type | Source |
|---|---|---|
| `episode_raw` | String | User/system input |
| `emotional_charge` | f64 | Emotional Tagger |
| `importance_score` | f64 | Importance Scorer |
| `decay_rate` | f64 | System (0.05) |
| `engram_type` | String | System ("episodic") |
| `compression_level` | i64 | System (0) |
| `contradiction_flag` | bool | System (false) |
| `embedding` | Vec<f32> | Embedding model (1536 dims) |

### Returns
The created Engram node (ID used for subsequent edge creation).

### Used By
- `Pipeline.ingest_memory()` — the primary ingestion pipeline
- Every new memory enters through this query

### Atomic Guarantee
All steps execute in a single `write_batch()` transaction. If any step fails, the entire operation rolls back — no orphaned partial data.

### Query Implementation
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
        // Single step: Create the Engram node WITH embedding directly on it
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
            ("embedding", embedding),
        ]))
        .returning(["engram"])
}
```

### Typical Query Patterns
```rust
// Primary ingestion: create engram with embedding in ONE call
let embedding = embed_text("User discussed Uganda ICT infrastructure").await?;
let request = create_full_engram(
    "User discussed Uganda ICT infrastructure".to_string(),
    0.7, 0.8, 0.05, "episodic".to_string(), 0, false,
    embedding,
);
let response = client.query(request).send().await?;
let engram_id: u64 = serde_json::from_value(response["engram"]["$id"].clone())?;
```

---

## Eliminated Queries

The following queries are **no longer needed** with direct-on-node embedding storage:

| Old Query | Reason Eliminated |
|---|---|
| `CreateEngramVectorText` | No separate `EngramVector` node — embedding lives on `Engram` |
| `CreateEngramVector` | Same — embeddings are a property, not a node |
| `ConnectEngramVector` | No `HasVector` edge — embedding is already on the `Engram` |

---

## Ingestion Call Order

```
Pipeline.ingest_memory(episode_raw):
  1. EmotionalTagger.tag(episode_raw)              → emotional_charge
  2. ImportanceScorer.score(episode_raw)           → importance_score
  3. embed_text(episode_raw)                        → embedding (client-side)
  4. CreateFullEngram(episode_raw, ..., embedding)  → engram_id
  5. ConceptExtractor.extract(episode_raw)          → [concepts...]
  6. For each concept:
       existing = GetConceptByName(concept.name)
       if !existing: CreateConcept(name, confidence) → concept_id
       ConnectRecalls(engram_id, concept_id)
       ConnectAbstraction(engram_id, concept_id)
       UpdateConceptSourceCount(concept_id)
  7. ContradictionDetector.check(engram_id)        → may call ConnectContradiction
```
