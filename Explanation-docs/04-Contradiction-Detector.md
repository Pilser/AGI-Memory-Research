# Contradiction Detector

## Trigger

**Fires in two places:**

1. **At ingestion time** — step 4, after concepts are linked. Checks if the newly created engram contradicts any existing engrams that share the same concepts.

2. **During consolidation** — the `consolidate()` loop re-scans all concepts for contradictions, because new engrams may have created indirect contradiction chains that weren't visible at ingestion time.

Triggered by: `Pipeline.ingest_memory()` (per-engram) and `Pipeline.consolidate()` (batch, via `detect_contradictions()`).

---

## What It Does

Detects when two engrams about the **same concept** have **opposing emotional valence** — meaning the AI has experienced contradictory information about the same topic. When detected, it:
1. Creates a `Contradicts` edge between the two engrams
2. Sets `contradiction_flag: true` on both engrams
3. This flag causes a **50% penalty** in the CRR resonance formula during retrieval

---

## DB Data It Reads

```
QUERY: DetectContradictionCandidates(concept_id)
```

| Field Read | Node | Type | Purpose |
|---|---|---|---|
| `ID` | `N::Engram` | `ID` | Identify engram pairs |
| `episode_raw` | `N::Engram` | `String` | Compare semantic content |
| `emotional_charge` | `N::Engram` | `F32` | Check for opposite signs |
| `importance_score` | `N::Engram` | `F32` | Prioritize high-importance contradictions |

Additionally, embeddings from `V::EngramVector` are retrieved to compute **semantic similarity** between candidate pairs (to confirm they're about the same thing, not just sharing a broad concept).

---

## DB Data It Writes

```
QUERY: ConnectContradiction(engram_a_id, engram_b_id)
```

| Written | Edge | Fields |
|---|---|---|
| `Contradicts` edge | `E::Contradicts` | `From: Engram, To: Engram` |

And via an `UpsertN` (not a separate query, but part of the flow):
- `contradiction_flag: true` on both engram nodes

---

## How It Works

### At Ingestion (per-engram check)

```
New engram arrives:
  episode_raw: "Remote work has destroyed team collaboration."
  emotional_charge: -0.65
  linked concepts: ["remote work productivity"]

Query: Get all engrams that also link to "remote work productivity"
  → Returns 47 engrams

Filter: Keep only engrams where sign(emotional_charge) ≠ sign(new_engram.emotional_charge)
  → Returns 12 engrams with positive valence

For each candidate pair:
  Compute semantic similarity between their embeddings
  If similarity > 0.7 (they're about the same thing):
    → Create Contradicts edge
    → Set contradiction_flag on both
```

### During Consolidation (batch re-scan)

```
For each concept in DB:
  Query: GetContradictionCandidates(concept_id)
  Group by emotional_charge sign
  Find cross-group pairs with high semantic similarity
  Create Contradicts edges
```

---

## Real Examples

```
Concept: "remote work productivity"

Engram A (3 weeks ago):
  "The team delivered 40% more features after going remote."
  emotional_charge: +0.7

Engram B (today):
  "Remote work has made collaboration impossible,
   we've never been less productive."
  emotional_charge: -0.65

→ Semantic similarity of embeddings: 0.89 (same topic)
→ Emotional divergence: |0.7 - (-0.65)| = 1.35 (strong opposition)
→ CONTRADICTION DETECTED

→ Create Contradicts edge: A ↔ B
→ Both flagged contradiction_flag = true
→ Future CRR queries: both penalized by 0.5×
```

```
Concept: "Python performance"

Engram A:
  "Python is fast enough for our API, we serve 10k req/s on a single process."
  emotional_charge: +0.6

Engram B:
  "Python is way too slow, we had to rewrite the hot path in Rust."
  emotional_charge: -0.4

→ Semantic similarity: 0.81
→ CONTRADICTION DETECTED
```

```
Concept: "coffee"

Engram A:
  "I love coffee, it's the best part of my morning."
  emotional_charge: +0.8

Engram B:
  "Tried decaf today. It was okay."
  emotional_charge: 0.0

→ Emotional divergence: 0.8 - 0.0 = 0.8
→ But semantic similarity: 0.42 (very different statements)
→ NOT a contradiction — decaf vs regular coffee are different topics
```

---

## Why Contradiction Detection Matters

Without contradiction detection, the AI confidently retrieves conflicting memories and presents both as truth:

**Without detector:**
> User: "How do you feel about remote work?"
> AI: "It boosted productivity by 40%! Also, it destroyed collaboration completely."
> User: "???"

**With detector:**
> User: "How do you feel about remote work?"
> AI: (CRR penalizes both contradicted engrams by 0.5×, so neither surfaces strongly)
> AI: "I have conflicting experiences with remote work. Some teams saw productivity gains, others struggled with collaboration. Would you like me to explore this contradiction?"

The consolidation loop can **resolve** contradictions by:
- Adjusting concept confidence scores
- Splitting the concept (e.g., "remote work productivity for engineering teams" vs. "remote work productivity for creative teams")
- Updating identity traits to reflect the ambiguity

---

## Recommended Approach: Hybrid Pipeline

> **This is the recommended approach.** Combines the speed of embedding filtering with the accuracy of LLM. The embedding filter narrows thousands of candidates to ~5-10 high-confidence pairs, then LLM makes the final decision.

### How It Works

```
Step 1: EMBEDDING FILTER (fast, free, local)
  For each concept:
    Get all engrams recalling this concept
    Filter: opposite emotional signs AND cosine_similarity > 0.70
    Result: ~5-10 candidate pairs (down from thousands)

Step 2: LLM VERIFICATION (accurate, few API calls)
  For each candidate pair:
    Ask LLM: "Do these contradict each other about the same topic?"
    Result: Confirmed contradiction or false positive
```

### Implementation

```rust
/// Detect contradictions using hybrid pipeline
pub async fn detect_contradictions(
    &self,
    concept_id: u64,
) -> Result<Vec<(u64, u64)>, Box<dyn std::error::Error>> {
    // Step 1: Embedding filter (fast, free)
    let candidates_request = get_engrams_recalling_concept_with_embeddings(concept_id);
    let candidates_response = self.client.query(candidates_request).send().await?;
    let engrams: Vec<EngramCandidateWithEmbedding> = serde_json::from_value(
        candidates_response["engrams"].clone()
    )?;

    // Filter: opposite emotional signs + high semantic similarity
    let mut candidate_pairs: Vec<(u64, u64, f64)> = vec![];
    for i in 0..engrams.len() {
        for j in (i + 1)..engrams.len() {
            let a = &engrams[i];
            let b = &engrams[j];
            let opposite_signs = a.emotional_charge.signum() != b.emotional_charge.signum();
            let emotional_gap = (a.emotional_charge - b.emotional_charge).abs();
            let sim = cosine_similarity(&a.embedding, &b.embedding);

            if opposite_signs && emotional_gap > 0.6 && sim > 0.70 {
                candidate_pairs.push((a.id, b.id, sim));
            }
        }
    }

    // Step 2: LLM verification (accurate, few calls)
    let mut confirmed_contradictions: Vec<(u64, u64)> = vec![];
    for (id_a, id_b, _) in candidate_pairs {
        let is_contradiction = self.llm_verify_contradiction(id_a, id_b).await?;
        if is_contradiction {
            confirmed_contradictions.push((id_a, id_b));
        }
    }

    Ok(confirmed_contradictions)
}

/// LLM verification for a candidate pair
async fn llm_verify_contradiction(
    &self,
    engram_a_id: u64,
    engram_b_id: u64,
) -> Result<bool, Box<dyn std::error::Error>> {
    let prompt = format!(
        "Compare these two statements. Do they contradict each other \
         about the same specific topic, or are they about different aspects? \
         Return JSON: {{\"contradicts\": true/false, \"explanation\": \"why or why not\"}}\n\n\
         Statement A: {}\n\
         Statement B: {}",
        get_engram_text(engram_a_id).await?,
        get_engram_text(engram_b_id).await?,
    );

    // Call LLM and parse response
    // ...
}
```

### When to Use Each Component

| Scenario | Approach | Why |
|----------|----------|-----|
| **At ingestion** (5-30 pairs) | LLM directly | Few pairs, accuracy matters most |
| **During consolidation** (thousands) | Embedding filter → LLM on top hits | Speed for bulk, accuracy for decision |

### Performance

| Scenario | Pairs After Filter | LLM Calls | Time |
|----------|-------------------|-----------|------|
| 1 new engram, 1 concept, 5 existing | ~2-3 | 2-3 | ~1s |
| 1 new engram, 5 concepts, 100 existing | ~10-15 | 10-15 | ~5s |
| Consolidation: 500 concepts, avg 10 each | ~50-100 | 50-100 | ~30s |
| Consolidation: 10,000 concepts | ~500-1000 | 500-1000 | ~5min |

The embedding filter reduces LLM calls by ~99% while maintaining accuracy.

---

## Implementation

For the actual Rust DSL implementation of queries mentioned in this doc, see:
- [Query Reference](../helixdb-embedded-queries.md)
- [Full Pipeline](../helixdb-embedded-pipeline.rs)
- [Schema Definition](../helixdb-embedded-schema.md)

For the resonance scoring that uses contradiction flags, see:
- [06-Resonance-Scorer.md](./06-Resonance-Scorer.md)
