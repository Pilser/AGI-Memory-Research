# Resonance Scorer (CRR)

## Trigger

**Fires at retrieval time** — whenever the AI needs to recall memories in response to a user query. This is the core retrieval algorithm. Every user interaction that requires memory lookup runs this.

Triggered by: `Pipeline.retrieve_memories()` in `pipeline.rs`.

---

## What It Does

Computes a **resonance score** for each candidate engram by combining five multiplicative factors. This is not "vector similarity search" — that's only factor 1. The resonance formula models **cognitive relevance** the way human memory works: emotional memories are stickier, recent memories are fresher, identity-aligned memories are more accessible, and contradicted memories are suppressed.

---

## The Formula

```
resonance(query, engram) =

    semantic_similarity(query_embedding, engram_embedding)   ← Factor 1
  × recency_weight(engram.timestamp, now)                    ← Factor 2
  × (1 + |engram.emotional_charge|)                          ← Factor 3
  × identity_alignment(engram, identity_graph)               ← Factor 4
  × (1 − engram.contradiction_flag × 0.5)                    ← Factor 5
```

### Factor Breakdown

#### Factor 1: Semantic Similarity

```
semantic_similarity = cosine(query_embedding, engram.embedding)
```

- **Source:** HelixDB `vector_search_with("Engram", "embedding", ...)` returns `$distance` (converted to similarity: `1.0 - distance`)
- **Range:** 0.0 (completely unrelated) to 1.0 (identical)
- **Role:** The baseline — "is this memory about the topic I'm asking about?"
- **ML involvement:** Embedding model (client-side) generates query vector; HelixDB does the cosine comparison via vector index on `Engram.embedding`

**Key simplification:** Embeddings are stored directly on Engram nodes. Vector search returns Engram nodes directly — no `::In<HasVector>` traversal needed.

#### Factor 2: Recency Weight

```
recency_weight = exp(−decay_rate × days_elapsed)
```

- **Source:** `engram.timestamp` field compared to current time
- **Range:** 0.01 (very old) to 1.0 (just created)
- **Role:** The Ebbinghaus forgetting curve. Memories fade exponentially unless reactivated.
- **ML involvement:** None — pure math. The `decay_rate` is a per-engram parameter (set at ingestion, default 0.05).

This is the same formula used in the consolidation loop to decide what to prune/compress/promote. At retrieval time it's used as a **weight**, not a **threshold** — old memories can still surface if they're highly semantically relevant and emotionally charged.

#### Factor 3: Emotional Amplification

```
emotional_boost = 1 + |engram.emotional_charge|
```

- **Source:** `engram.emotional_charge` (scored by Emotional Tagger)
- **Range:** 1.0 (neutral) to 2.0 (extremely emotional, positive or negative)
- **Role:** Emotional memories are stickier. Both very positive and very negative experiences get up to 2× boost. Neutral experiences pass through unchanged.
- **ML involvement:** Uses the Emotional Tagger's output (pre-computed at ingestion)

This models the **amygdala effect** — the human brain's tendency to consolidate emotional memories more strongly than neutral ones.

#### Factor 4: Identity Alignment

```
identity_alignment = how well the engram's concepts match the AI's identity traits
```

- **Source:** The `Identity` subgraph — traits like `{trait: "knows African tech", value: 0.8, stability: 0.9}`
- **Range:** 0.0 (completely misaligned) to 1.0 (strongly aligned)
- **Role:** An AI that "identifies as" knowledgeable about East African tech will have higher resonance for memories about Uganda ICT Hub than for memories about, say, Canadian hockey — even if semantic similarity is equal.
- **ML involvement:** The alignment is computed by:
  1. Walking the engram → concept → identity chain via graph traversal
  2. Computing cosine similarity between the engram's embedding and each identity trait's embedding
  3. Weighting by trait `value` and `stability`

This models **self-schema** — the human tendency to remember things that fit our self-concept more readily than things that don't.

#### Factor 5: Contradiction Penalty

```
contradiction_penalty = 1 − (contradiction_flag × 0.5)
```

- **Source:** `engram.contradiction_flag` (set by Contradiction Detector)
- **Range:** 0.5 (contradicted) to 1.0 (not contradicted)
- **Role:** If the AI has conflicting information about a topic, both sides are suppressed. The AI doesn't confidently retrieve either until the contradiction is resolved.
- **ML involvement:** Uses the Contradiction Detector's output (pre-computed)

---

## DB Data It Reads

### Step 1: Vector Search (HelixDB)

```
QUERY: SearchEngramsFull(query, limit, min_importance, exclude_contradictions)
```

| Field Read | Node | Purpose |
|---|---|---|
| `ID` | `N::Engram` | Identify the engram |
| `episode_raw` | `N::Engram` | For identity alignment computation |
| `emotional_charge` | `N::Engram` | Factor 3 |
| `importance_score` | `N::Engram` | Pre-filter threshold |
| `engram_type` | `N::Engram` | Optional filter by type |
| `timestamp` | `N::Engram` | Factor 2 (recency) |
| Cosine distance | SearchV result | Factor 1 (semantic similarity) |

### Step 2: Identity Graph

```
QUERY: GetAllIdentity()
```

| Field Read | Node | Purpose |
|---|---|---|
| `ID` | `N::Identity` | Identify the trait |
| `trait` | `N::Identity` | Trait name |
| `value` | `N::Identity` | How strongly the AI holds this trait |
| `stability` | `N::Identity` | Resistance to change |

### Step 3: Per-engram identity alignment

```
QUERY: GetCognitiveChain(engram_id)
```

| Field Read | Source | Purpose |
|---|---|---|
| Concepts linked to engram | `N::Concept` via `::Out<Recalls>` | Compare concepts to identity traits |
| Identity traits linked to concepts | `N::Identity` via `::Out<Defines>` | Direct alignment check |

---

## DB Data It Writes

```
QUERY: ActivateEngram(engram_id)
```

| Field Written | Node | Purpose |
|---|---|---|
| `activation_count` | `N::Engram` | Incremented by 1 for each recalled engram |

This is critical: every time an engram is recalled, its `activation_count` increases, which **slows its decay** in future consolidation cycles. Frequently recalled memories resist forgetting — just like human memory.

---

## How It Works — Full Retrieval Walkthrough

```
User query: "What do I know about East African tech?"

Step 1: HelixDB Vector Search
  Embed("What do I know about East African tech?") → query_vector[1536]
  vector_search_with("Engram", "embedding", query_vector, limit=20) → top-20 Engram nodes DIRECTLY
  (No ::In<HasVector> traversal needed — embedding is on the Engram node)
  Filter: importance >= 0.0, contradiction_flag != true
  Returns: 18 candidates (2 filtered out by contradiction)

Step 2: Fetch Identity Graph
  GetAllIdentity() → [
    {trait: "knows African tech", value: 0.85, stability: 0.9},
    {trait: "prefers Rust over Python", value: 0.6, stability: 0.4},
    {trait: "believes remote work is productive", value: 0.7, stability: 0.6},
    ...
  ]

Step 3: Compute Resonance for each candidate

Candidate A: "The Uganda ICT Hub in Kampala is attracting VC..."
  semantic_sim:  0.88  ← SearchV cosine
  recency:       0.92  ← created 3 days ago, decay_rate=0.05
  emotional:     1.75  ← |0.75| + 1 (strong positive)
  identity:      0.91  ← strongly aligns with "knows African tech" (0.85)
  contradiction: 1.0   ← not contradicted

  RESONANCE = 0.88 × 0.92 × 1.75 × 0.91 × 1.0 = 1.29

Candidate B: "Nairobi-based funds are expanding to Kigali..."
  semantic_sim:  0.82
  recency:       0.85  ← created 2 weeks ago
  emotional:     1.6   ← |0.6| + 1
  identity:      0.88  ← aligns with African tech
  contradiction: 1.0

  RESONANCE = 0.82 × 0.85 × 1.6 × 0.88 × 1.0 = 0.98

Candidate C: "The Canadian tech scene in Toronto is growing..."
  semantic_sim:  0.45  ← about tech, but not East African
  recency:       0.95  ← very recent
  emotional:     1.3   ← mild positive
  identity:      0.15  ← doesn't align with "knows African tech"
  contradiction: 1.0

  RESONANCE = 0.45 × 0.95 × 1.3 × 0.15 × 1.0 = 0.083

Candidate D: "East African tech is still too donor-dependent..."
  semantic_sim:  0.86
  recency:       0.70  ← created 2 months ago
  emotional:     1.45  ← |0.45| + 1 (mild concern)
  identity:      0.80
  contradiction: 0.5   ← CONTRADICTED by Candidate A

  RESONANCE = 0.86 × 0.70 × 1.45 × 0.80 × 0.5 = 0.35

Step 4: Sort by Resonance
  A: 1.29  ← TOP — recent, emotional, identity-aligned
  B: 0.98
  D: 0.35  ← suppressed by contradiction penalty
  C: 0.083 ← irrelevant

Step 5: Activate top results
  ActivateEngram(A.id) → activation_count++
  ActivateEngram(B.id) → activation_count++
  (Skip C and D — below relevance threshold)

Step 6: Inject into LLM context
  System prompt ← "Relevant memories: {A.episode_raw}\n{B.episode_raw}"
```

---

## Recommended Approach: Multiplicative Formula

> **This is the recommended approach.** The Resonance Scorer is pure math — a deterministic Rust function with no weights to learn. Personalization comes from the **Stimulation Layer's learnable edge weights** (see [07-Stimulation-Layer.md](./07-Stimulation-Layer.md)), not from making the scorer itself learn.

### Why Not a Learned Scorer?

| Approach | Complexity | Personalization | Recommendation |
|----------|------------|-----------------|----------------|
| **Multiplicative formula** | Simple, deterministic | Comes from Stimulation Layer | **Recommended** |
| **Learned scorer (XGBoost/NN)** | More complex | Redundant with Stimulation Layer | Not needed |

The Stimulation Layer already has 9 learnable α weights that personalize how activation YOUR retrieval patterns. Adding a learned resonance scorer on top would be redundant — two systems trying to personalize the same thing.

### Implementation

```rust
/// Compute resonance score using the 5-factor multiplicative formula
/// This is pure math — no learned weights
pub fn compute_resonance(
    semantic_sim: f64,        // Factor 1: from vector search (1.0 - distance)
    days_elapsed: f64,        // Factor 2: from engram.timestamp
    emotional_charge: f64,    // Factor 3: from Emotional Tagger
    identity_alignment: f64,  // Factor 4: from graph traversal
    contradiction_flag: bool,  // Factor 5: from Contradiction Detector
    decay_rate: f64,          // per-engram parameter (default: 0.05)
) -> f64 {
    // Factor 1: semantic similarity (baseline)
    let semantic = semantic_sim;

    // Factor 2: recency weight (Ebbinghaus forgetting curve)
    let recency = (-decay_rate * days_elapsed).exp().max(0.01);

    // Factor 3: emotional amplification (amygdala effect)
    let emotional_boost = 1.0 + emotional_charge.abs();

    // Factor 4: identity alignment (self-schema)
    let identity = identity_alignment;

    // Factor 5: contradiction penalty
    let contradiction_factor = if contradiction_flag { 0.5 } else { 1.0 };

    // Multiplicative combination
    semantic * recency * emotional_boost * identity * contradiction_factor
}

/// Compute identity alignment for an engram
pub async fn compute_identity_alignment(
    client: &Client,
    engram_id: u64,
) -> Result<f64, Box<dyn std::error::Error>> {
    // 1. Get engram → concepts → identity chain
    let chain_request = get_cognitive_chain(engram_id);
    let chain_response = client.query(chain_request).send().await?;

    // 2. Compute alignment between engram concepts and identity traits
    // (cosine similarity between embeddings, weighted by trait value and stability)
    // ...

    Ok(alignment_score)
}
```

### Where Personalization Actually Happens

| Component | What It Personalizes | How |
|-----------|---------------------|-----|
| **Stimulation Layer** (doc 07) | How activation spreads through the graph | 9 learnable α weights (Adam optimizer) |
| **Resonance Scorer** (this doc) | Nothing — it's deterministic | Pure math formula |
| **Learnable Edge Weights** (doc 08) | YOUR retrieval dynamics | Real-time SGD from usage feedback |

The Resonance Scorer stays simple and explainable. The Stimulation Layer's learnable weights handle personalization.

---

## Thresholds

| Parameter | Default | Meaning |
|---|---|---|
| `limit` | 20 | Max candidates from SearchV (use a generous limit, resonance sorts them) |
| `min_importance` | 0.0 | Filter out trivial engrams before scoring. Set to 0.2–0.3 if too many noise candidates. |
| `exclude_contradictions` | true | Skip contradicted engrams entirely (faster) rather than penalizing their score. |
| `min_resonance` | 0.05 | Don't return engrams with resonance below this (they're effectively irrelevant) |
| `max_results` | 10 | Cap final returned results to avoid flooding the LLM context window |
| `activation_threshold` | 0.3 | Only bump activation_count for engrams with resonance above this |

---

## Why This Beats RAG

| RAG | CRR |
|---|---|
| `cosine_sim(query, chunk)` | `semantic × recency × emotion × identity × !contradiction` |
| Stateless — retrieves same chunks for same query every time | Stateful — activation_count changes future resonance (things you recall often become easier to recall) |
| No concept of "I believe X" vs "I used to believe Y" | Identity alignment and contradiction penalty model belief evolution |
| A document from 3 years ago about depression-era economics has same retrieval chance as one about your current project | Recency weight naturally deprioritizes old knowledge unless it's identity-critical or frequently activated |
| All chunks are equal | Emotional memories get up to 2× boost |
| No contradiction handling | Contradicted memories are suppressed until resolved |

---

## Implementation

For the actual Rust DSL implementation of queries mentioned in this doc, see:
- [Query Reference](../helixdb-embedded-queries.md)
- [Full Pipeline](../helixdb-embedded-pipeline.rs)
- [Retrieval Layer](../helixdb-embedded-retrieval.md)
- [Schema Definition](../helixdb-embedded-schema.md)

For personalization (learnable edge weights that adapt to user), see:
- [07-Stimulation-Layer.md](./07-Stimulation-Layer.md) — spreading activation with learnable weights
- [08-Learnable-Edge-Weights.md](./08-Learnable-Edge-Weights.md) — Adam optimizer implementation
