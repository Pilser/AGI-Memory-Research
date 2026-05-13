# Contradiction Queries — Detecting Conflicting Knowledge

These queries support the Contradiction Detector (ML model #4). They find and track engrams that convey opposing information about the same concept.

---

## 1. DetectContradictionCandidates

### What It Does
For a given concept, returns all engrams that recall it — with just enough data to compare emotional charges. The application code then groups engrams by emotional sign (positive vs negative) and identifies contradictory pairs.

### Parameters
| Param | Type | Purpose |
|---|---|---|
| `concept_id` | ID | The concept to check for contradictions among its engrams |

### Returns
    [{ ID, episode_raw, emotional_charge, importance_score }, ...]

### Used By
- Contradiction Detector (ML) — both at ingestion and during consolidation
- The returned `emotional_charge` values are grouped by sign, then cross-compared

### Rust Post-Processing
    let engrams = client.query("DetectContradictionCandidates", &json!({
        "concept_id": concept_id
    })).await?;

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
            let sim = cosine_similarity(pos.embedding, neg.embedding);
            if sim > 0.7 {  // they're about the same topic
                client.query("ConnectContradiction", &json!({
                    "engram_a_id": pos.id,
                    "engram_b_id": neg.id
                })).await?;
            }
        }
    }

---

## 2. GetContradictedEngrams

### What It Does
Returns all engrams that are actively flagged as contradicted (`contradiction_flag = true` AND have outgoing `Contradicts` edges). Used to monitor contradiction health and for resolution prioritization.

### Parameters
None.

### Returns
    [{ ID, episode_raw, engram_type }, ...]

### Used By
- Consolidation loop — prioritizes which contradictions to resolve first
- Memory health stats — gives the `contradiction_count` metric
- UI/dashboard — shows "unresolved conflicts" in the knowledge graph

---

## Contradiction Resolution Flow

    1. GetContradictedEngrams() → list of flagged engrams
    2. For each flagged engram:
       GetContradictions(engram_id) → both directions
       GetCognitiveChain(engram_id) → concepts and identity involved

    3. Resolution strategies (in order):
       a. SPLIT THE CONCEPT:
          If the contradiction is about two different sub-topics,
          trigger mitosis on the shared concept to separate them.

       b. ADJUST CONFIDENCE:
          Reduce concept.confidence to reflect uncertainty.
          Lower confidence → lower identity impact.

       c. TEMPORAL RESOLUTION:
          If newer engram contradicts older AND older is decaying,
          newer evidence wins. Older engram gets pruned or compressed.

       d. IDENTITY CHECK:
          If contradiction involves a high-stability identity trait,
          don't resolve automatically. Flag for human review or
          wait for more evidence.
