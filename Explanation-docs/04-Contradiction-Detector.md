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

## Possible Model Types

### Option A: LLM with structured output (OpenAI, Anthropic)

```
Prompt: "Compare these two statements. Do they contradict each other
         about the same specific topic, or are they about different
         aspects? Return JSON:

         {
           "contradicts": true/false,
           "semantic_similarity": 0.0-1.0,
           "explanation": "why or why not"
         }

         Statement A: {engram_a.episode_raw}
         Statement B: {engram_b.episode_raw}"
```

- **Pros:** Best accuracy, understands nuance and domain context
- **Cons:** Slow if checking many pairs (47 engrams × 12 candidates = 564 comparisons)

### Option B: Embedding comparison (fast, local)

- Embed both engram texts (reuse existing `EngramVector` from DB)
- Compute cosine similarity
- If similarity > threshold (e.g., 0.75) AND opposite emotional signs → contradiction

```rust
fn detect_contradiction(a: &Engram, b: &Engram) -> bool {
    let sim = cosine_similarity(&a.embedding, &b.embedding);
    let opposite_signs = a.emotional_charge.sign() != b.emotional_charge.sign();
    let emotional_gap = (a.emotional_charge - b.emotional_charge).abs();

    sim > 0.70 && opposite_signs && emotional_gap > 0.6
}
```

- **Pros:** Very fast, no additional API calls, embeddings already stored
- **Cons:** Embedding similarity ≠ contradiction detection; two texts can be semantically similar but not contradictory (e.g., two positive statements about the same thing)

### Option C: Cross-encoder / NLI model (ONNX / Candle)

- Use a Natural Language Inference model (e.g., RoBERTa fine-tuned on MNLI)
- Input: premise (engram A text) + hypothesis (engram B text)
- Output: entailment / neutral / contradiction

- **Pros:** Specifically designed for contradiction detection, high accuracy
- **Cons:** One forward pass per pair, needs ONNX export and `ort` runtime in Rust

### Option D: Hybrid pipeline

1. **Embedding filter** (fast, Option B) → narrow down to candidates with cosine similarity > 0.7 AND opposite emotional signs
2. **LLM or NLI model** (accurate, Option A or C) → only on the filtered pairs
3. This reduces cost: from N×M comparisons to ~5-10 LLM calls per new engram

---

## Performance Considerations

| Scenario | Pairs to Check | Strategy |
|---|---|---|
| 1 new engram, 1 concept, 5 existing engrams | 5 | All pairs via LLM — cheap |
| 1 new engram, 5 concepts, 100 total existing engrams | ~30 (after emotional sign filter) | Embedding filter → LLM top hits |
| Consolidation: 500 concepts, avg 10 engrams each | ~2500 concept groups | Batch embedding comparison, LLM only on high-confidence hits |
| Consolidation: 10,000 concepts | ~50,000 concept groups | Scheduled: process oldest concepts first, rotate daily |
