# Read Queries — Reading Data from DB

These queries retrieve engrams, concepts, identity traits, and their relationships. Called at retrieval time, during consolidation, and for inspection.

> **Key simplification:** Since embeddings are stored directly on `Engram` nodes, there's no need for `GetEngramWithVector` — the embedding comes with every engram read.

---

## 1. GetEngram

### What It Does
Returns a single engram by its ID — including its `embedding` field directly on the node.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | u64 |

### Used By
- Any pipeline step that needs to inspect a specific engram
- Debugging and testing

### Query Implementation
```rust
use helix_db::dsl::prelude::*;

#[query]
fn get_engram(engram_id: u64) -> ReadBatch {
    read_batch()
        .var_as("engram", g()
            .n(NodeRef::id(engram_id))
            .value_map(Some(vec![
                "$id", "episode_raw", "emotional_charge", "importance_score",
                "decay_rate", "activation_count", "timestamp", "engram_type",
                "compression_level", "contradiction_flag", "embedding",
            ])))
        .returning(["engram"])
}
```

### Typical Query Patterns
```rust
let request = get_engram(engram_id);
let response = client.query(request).send().await?;
let engram: EngramData = serde_json::from_value(response["engram"].clone())?;
// engram.embedding is available directly — no traversal needed
```

---

## 2. GetEngramsByType

### What It Does
Returns all engrams of a given type (e.g., all "episodic", all "semantic", all "procedural").

### Parameters
| Param | Type |
|---|---|
| `engram_type` | String ("episodic" / "semantic" / "procedural") |

### Used By
- Consolidation loop — processes episodic engrams for decay
- Stats and inspection

### Query Implementation
```rust
#[query]
fn get_engrams_by_type(engram_type: String) -> ReadBatch {
    read_batch()
        .var_as("engrams", g()
            .n_with_label_where("Engram", SourcePredicate::eq("engram_type", engram_type))
            .value_map(Some(vec![
                "$id", "episode_raw", "emotional_charge", "importance_score", "timestamp",
            ])))
        .returning(["engrams"])
}
```

---

## 3. GetConceptByName

### What It Does
Looks up a concept by its `name` field. Requires an INDEX on `Concept.name`. Returns the concept if found, otherwise empty. This is the deduplication check — before creating a new concept, check if one with the same name already exists.

### Parameters
| Param | Type |
|---|---|
| `name` | String |

### Used By
- `Pipeline.ingest_memory()` — deduplication check before `CreateConcept`
- Concept Extractor — finding existing concepts to reuse

### Query Implementation
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

---

## 4. GetRecalledConcepts

### What It Does
Returns all concepts that an engram recalls. Traverses `::Out<Recalls>` from the engram.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | u64 |

### Used By
- Contradiction Detector — finds what concepts an engram relates to
- Identity alignment — traces engram → concepts chain
- Debugging — "what does this memory know about?"

### Query Implementation
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

---

## 5. GetEngramsRecallingConcept

### What It Does
Returns all engrams that recall a given concept. Traverses `::In<Recalls>` from the concept. The inverse of `GetRecalledConcepts`.

### Parameters
| Param | Type |
|---|---|
| `concept_id` | u64 |

### Used By
- Mitosis Splitter — gets all engrams linked to a concept for clustering
- Contradiction Detector — gets all engrams about a concept to check for opposing emotions
- `UpdateConceptSourceCount` — counts how many engrams feed a concept

### Query Implementation
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

---

## 6. GetContradictions

### What It Does
Returns both outgoing and incoming `Contradicts` edges for an engram. Since contradiction is symmetric, this tells you all engrams that contradict or are contradicted by this one.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | u64 |

### Returns
```json
{
    "outgoing": [{ "id": 42, "episode_raw": "...", "emotional_charge": 0.8 }],
    "incoming": [{ "id": 57, "episode_raw": "...", "emotional_charge": -0.6 }]
}
```

### Used By
- Contradiction resolution (consolidation loop)
- Debugging contradiction chains

### Query Implementation
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

---

## 7. GetIdentityAlignment

### What It Does
For a given engram, walks the full cognitive chain (`Engram → Recalls → Concept → Defines → Identity`) and returns all identity traits that this engram's concepts define. This is Factor 4 of the resonance formula.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | u64 |

### Returns
All Identity nodes reachable from the engram via its recalled concepts.

### Used By
- Resonance Scorer — computes `identity_alignment()` for Factor 4
- Identity crystallization — determines which traits a new engram affects

### Query Implementation
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

---

## 8. GetCognitiveChain

### What It Does
The full cognitive trace for an engram: returns the engram itself, its concepts, and the identity traits those concepts define — all in one query with property projection for clean output.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | u64 |

### Returns
```json
{
    "engram":     { "episode_raw": "...", "engram_type": "episodic", "emotional_charge": 0.7 },
    "concepts":   [{ "name": "Uganda ICT", "confidence": 0.85 }],
    "identities": [{ "trait": "knows East African tech", "value": 0.85, "stability": 0.9 }]
}
```

### Used By
- Resonance Scorer — full context for scoring
- Debugging and visualization — "show me the full cognitive picture of this memory"
- `Pipeline.update_identity_from_experience()` — updates identity traits based on engram impact

### Query Implementation
```rust
#[query]
fn get_cognitive_chain(engram_id: u64) -> ReadBatch {
    read_batch()
        .var_as("engram", g()
            .n(NodeRef::id(engram_id))
            .value_map(Some(vec![
                "$id", "episode_raw", "emotional_charge", "importance_score", "engram_type",
            ])))
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

## Eliminated Queries

| Old Query | Reason Eliminated |
|---|---|
| `GetEngramWithVector` | No longer needed — `embedding` is a property on every `Engram` node, returned automatically by `GetEngram` |

---

## Read Query Call Patterns

### Retrieval Flow
```
Pipeline.retrieve_memories(query):
  1. embed_text(query)                               → query_embedding (client-side)
  2. SearchEngramsFull(query_embedding, limit, ...)   → candidates (searches Engram.embedding directly)
  3. GetAllIdentity()                                 → identity traits
  4. For each top candidate:
       GetCognitiveChain(engram_id)                   → alignment data
       Compute resonance in Rust
  5. ActivateEngram(top_candidates)
```

### Consolidation Flow
```
Pipeline.consolidate():
  1. GetEngramsForDecayCheck("episodic")              → candidates for decay
  2. For each engram:
       Compute retention in Rust
       CompressEngram / PruneEngram / PromoteToSemantic
  3. CheckMitosisCandidates(min_count)                → overloaded concepts
  4. For each concept:
       GetEngramsRecallingConcept(concept_id)          → engram data (embedding on node)
       Cluster in Rust → split concept
  5. Detect contradictions batch scan
```
