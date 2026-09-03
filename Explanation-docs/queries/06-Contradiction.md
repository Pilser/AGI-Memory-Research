# Contradiction Queries — Detecting Conflicting Knowledge

These queries support the Contradiction Detector (ML model #4). They find and track engrams that convey opposing information about the same concept.

---

## 1. DetectContradictionCandidates

### What It Does
For a given concept, returns all engrams that recall it — with just enough data to compare emotional charges. The application code then groups engrams by emotional sign (positive vs negative) and identifies contradictory pairs.

### Parameters
| Param | Type | Purpose |
|---|---|---|
| `concept_id` | u64 | The concept to check for contradictions among its engrams |

### Returns
```json
{
    "engrams": [
        { "$id": 42, "episode_raw": "Remote work boosted productivity", "emotional_charge": 0.8, "importance_score": 0.7 },
        { "$id": 57, "episode_raw": "Remote work caused isolation", "emotional_charge": -0.6, "importance_score": 0.5 }
    ]
}
```

### Used By
- Contradiction Detector (ML) — both at ingestion and during consolidation
- The returned `emotional_charge` values are grouped by sign, then cross-compared

### Query Implementation
```rust
use helix_db::dsl::prelude::*;

#[query]
fn detect_contradiction_candidates(concept_id: u64) -> ReadBatch {
    read_batch()
        .var_as("engrams", g()
            .n(NodeRef::id(concept_id))
            .in_(Some("Recalls"))
            .value_map(Some(vec![
                "$id", "episode_raw", "emotional_charge", "importance_score",
            ])))
        .returning(["engrams"])
}
```

### Typical Query Patterns
```rust
let request = detect_contradiction_candidates(concept_id);
let response = client.query(request).send().await?;
let engrams: Vec<EngramData> = serde_json::from_value(response["engrams"].clone())?;

let mut positive = vec![];
let mut negative = vec![];

for e in &engrams {
    if e.emotional_charge > 0.3 {
        positive.push(e);
    } else if e.emotional_charge < -0.3 {
        negative.push(e);
    }
}

// Cross-compare positive vs negative
for pos in &positive {
    for neg in &negative {
        // Compute embedding similarity between pos and neg
        // (embeddings available directly on engram nodes)
        let sim = cosine_similarity(&pos.embedding, &neg.embedding);
        if sim > 0.7 {  // they're about the same topic
            let request = connect_contradiction(pos.id, neg.id);
            client.query(request).send().await?;
        }
    }
}
```

---

## 2. GetContradictedEngrams

### What It Does
Returns all engrams that are actively flagged as contradicted (`contradiction_flag = true` AND have outgoing `Contradicts` edges). Used to monitor contradiction health and for resolution prioritization.

### Parameters
None.

### Returns
```json
{
    "engrams": [
        { "$id": 42, "episode_raw": "Remote work boosted productivity", "engram_type": "episodic" },
        { "$id": 57, "episode_raw": "Remote work caused isolation", "engram_type": "episodic" }
    ]
}
```

### Used By
- Consolidation loop — prioritizes which contradictions to resolve first
- Memory health stats — gives the `contradiction_count` metric
- UI/dashboard — shows "unresolved conflicts" in the knowledge graph

### Query Implementation
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

## Contradiction Resolution Flow

```rust
// 1. Get all contradicted engrams
let contradicted = get_contradicted_engrams().await?;

// 2. For each flagged engram:
for engram in contradicted {
    let contradictions = get_contradictions(engram.id).await?;
    let chain = get_cognitive_chain(engram.id).await?;
    
    // 3. Resolution strategies (in order):
    //    a. SPLIT THE CONCEPT:
    //       If the contradiction is about two different sub-topics,
    //       trigger mitosis on the shared concept to separate them.
    //
    //    b. ADJUST CONFIDENCE:
    //       Reduce concept.confidence to reflect uncertainty.
    //       Lower confidence → lower identity impact.
    //
    //    c. TEMPORAL RESOLUTION:
    //       If newer engram contradicts older AND older is decaying,
    //       newer evidence wins. Older engram gets pruned or compressed.
    //
    //    d. IDENTITY CHECK:
    //       If contradiction involves a high-stability identity trait,
    //       don't resolve automatically. Flag for human review or
    //       wait for more evidence.
}
```
