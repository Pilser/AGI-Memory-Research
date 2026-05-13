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
semantic_similarity = cosine(Embed(query), engram_vector)
```

- **Source:** HelixDB `SearchV` returns cosine similarity scores
- **Range:** 0.0 (completely unrelated) to 1.0 (identical)
- **Role:** The baseline — "is this memory about the topic I'm asking about?"
- **ML involvement:** The embedding model (OpenAI/Gemini) generates the vectors; HelixDB does the cosine comparison server-side

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

Step 1: HelixDB SearchV
  Embed("What do I know about East African tech?") → query_vector[1536]
  SearchV<EngramVector>(query_vector, limit=20) → top-20 candidate vectors
  ::In<HasVector> → traverse back to 20 engram nodes
  ::WHERE(importance >= 0.0, contradiction_flag != true) → apply filters
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

## Possible Model Types

### The Resonance Scorer is NOT a trained model

It's pure math — a Rust function with no weights to learn. The "intelligence" comes from the four other ML models whose outputs it combines:

| Factor | Powered By | Trained? |
|---|---|---|
| Semantic similarity | Embedding model (OpenAI/Gemini) | Pre-trained |
| Recency weight | Math (exponential decay) | Not ML |
| Emotional amplification | Emotional Tagger output | Custom ML (see doc 01) |
| Identity alignment | Graph traversal + embedding comparison | Uses Identity graph (built over time) |
| Contradiction penalty | Contradiction Detector output | Custom ML or rule-based (see doc 04) |

### However — you can make it learn

A **learned resonance scorer** would replace the multiplicative formula with a model that learns weights from user feedback:

```
Input features:
  - semantic_similarity
  - days_elapsed
  - |emotional_charge|
  - identity_alignment
  - contradiction_flag
  - activation_count (how often recalled in the past)
  - engram_type (episodic vs semantic vs procedural)

Label:
  - Did the user engage with this memory? (yes/no)
  - Did the user reference it later? (yes/no)
  - Did the user explicitly mark it as "important" or "irrelevant"?

Model:
  - XGBoost or small neural network
  - Trained on your own interaction data over time
  - Outputs a single resonance score

→ The AI learns what YOU consider relevant, not what a formula says
```

This is Phase 2+ material — the multiplicative formula is Phase 1.

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
