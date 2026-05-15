# Stimulation Layer — From Memory Query to Memory Activation

## The Fundamental Shift

The current CRR retrieval model (`pipeline.rs:171`, `06-Resonance-Scorer.md`) works like this:

```
Query arrives → SearchV finds candidates → 5-factor formula scores them → return sorted list
```

That is a **query layer**. A stimulus goes in, ranked results come out. It is correct, deterministic, and fast. It is also **not how biological memory works**.

Biological memory is not queried. It is **stimulated**. A stimulus perturbs a field of interconnected traces, activation spreads through weighted associations, and memories **surface into consciousness** when their accumulated activation crosses a threshold. One memory triggers another — the associative cascade. A second wave of weaker associations follows. Activation reverberates through recurrent loops. The process is noisy, state-dependent, and never produces exactly the same result twice.

This document describes how to evolve the MNEMOS retrieval architecture from the existing **query layer** (Phase 1 CRR formula) into a **stimulation layer** — a dynamical system where memory feels like *remembering*, not like *searching*.

---

## The Concept: Spreading Activation on a Cognitive Graph

### Current Model (Static Retrieval)

```
┌──────────┐     ┌───────────┐     ┌──────────────┐     ┌─────────┐
│  Query   │ ──▶ │  HelixDB  │ ──▶ │  CRR Formula │ ──▶ │  Sorted │
│  Text    │     │  SearchV  │     │  (5 factors) │     │  List   │
└──────────┘     └───────────┘     └──────────────┘     └─────────┘
```

Single pass. Deterministic. Entire function completes before LLM sees any result.

### Proposed Model (Dynamical Activation)

```
                   ┌──────────────────────────────────────────┐
                   │          STIMULATION LAYER                │
                   │                                           │
  ┌──────────┐    │  STEP 1: INJECTION                        │
  │ Stimulus │───▶│    Query embedding seeds initial nodes     │
  │ (query,  │    │    via SearchV (as today)                  │
  │  context,│    │                                           │
  │  mood)   │    │  STEP 2: SPREADING WAVE                   │
  └──────────┘    │    Activation propagates through edges:    │
                   │      HasVector:    α₁ × cosine_sim        │
                   │      Recalls:      α₂ (engram→concept)    │
                   │      AbstractsTo:  α₃ (engram→concept)    │
                   │      Reinforces:   α₄ (engram→engram)     │
                   │      TemporalSeq:  α₅ × time_weight       │
                   │      Contradicts:  −α₆ (negative weight)  │
                   │      Defines:      α₇ (concept→identity)  │
                   │      SpawnedFrom:  α₈ (parent→child)      │
                   │    Each α is a per-edge-type weight        │
                   │                                           │
                   │  STEP 3: RECURRENT FEEDBACK               │
                   │    Activated engrams re-inject their       │
                   │    concepts as secondary seeds             │
                   │    → cascading activation wave             │
                   │    → "Oh, that reminds me of..."           │
                   │                                           │
                   │  STEP 4: DECAY AND SURFACING              │
                   │    Activation decays per timestep          │
                   │    Nodes above threshold → surfaced        │
                   │    Surfaced nodes → re-injected (goto 3)   │
                   │    Iterate until energy falls below τ      │
                   │                                           │
                   │  OUTPUT: dynamically surfaced engram set   │
                   └──────────────────────────────────────────┘
```

The LLM doesn't receive "search results." It receives a **wave of activations** that arrived together, with associative connections between them. The ordering isn't "most relevant first" — it's "what surfaced through the natural dynamics of this cognitive graph given this stimulus."

---

## The Algorithm

### Core Loop

```
Given: stimulus embedding q, initial seed limit L, max iterations M,
       energy decay γ, surfacing threshold τ, per-edge weights α[8]

1. SEED PHASE
   seeds = SearchV<EngramVector>(q, L)::In<HasVector>
   For each seed s:
       activation[s] = cosine_sim(q, s.vector) × (1 + |s.emotional_charge|)
       activation[s] *= recency_weight(s.timestamp)   // Factor 2 from CRR
       activation[s] *= (1 − s.contradiction_flag × 0.5)  // Factor 5 from CRR

2. SPREADING PHASE (iterate up to M times)
   For iteration t in 0..M:
       new_activation = {}  // fresh map for this timestep

       For each node n with activation[n] > τ:

           // 2a. Spread to outgoing neighbors
           For each edge e = (n → neighbor) of type T:
               transfer = activation[n] × α[T] × edge_specific_weight(e)
               new_activation[neighbor] += transfer

           // 2b. Spread to incoming neighbors (bidirectional flow)
           For each edge e = (predecessor → n) of type T:
               transfer = activation[n] × α[T] × edge_specific_weight(e)
               new_activation[predecessor] += transfer

           // 2c. Recurrent: activated engram re-injects its concepts
           If n is Engram AND t < max_iterations:
               // The engram "reminds" the system of its concepts
               For each concept c linked via Recalls or AbstractsTo:
                   new_activation[c] += activation[n] × β_recurrent

       // 2d. Identity dampening (stability gates activation)
       For each Identity node i:
           new_activation[i] *= i.stability  // stable traits absorb slowly

       // 2e. Merge + decay
       For each node n:
           activation[n] = (activation[n] + new_activation[n]) × γ

   // 2f. Capture surfaced engrams at each timestep
   surfaced[t] = {n | n is Engram AND activation[n] > τ}

3. OUTPUT
   Return surfaced grouped by timestep, maintaining wave ordering:
     Wave 1: directly triggered by stimulus
     Wave 2: triggered by Wave 1's associations
     Wave 3: triggered by Wave 2's associations
     ...
```

### Edge-Specific Weights

These are the α weights controlling how much activation each edge type transmits. Phase 1 uses hand-tuned defaults; Phase 2 makes them learnable.

| Index | Edge Type | Default α | Direction | Rationale |
|---|---|---|---|---|
| α₁ | `HasVector` | 0.85 | Engram ↔ EngramVector | Vector bridge — high weight because it carries semantic content |
| α₂ | `Recalls` | 0.70 | Engram → Concept | Primary association — engrams recall concepts |
| α₃ | `AbstractsTo` | 0.50 | Engram → Concept | Weaker than Recalls — abstraction is deliberative |
| α₄ | `Reinforces` | 0.60 | Engram ↔ Engram | Mutual reinforcement — supporting memories boost each other |
| α₅ | `TemporalSequence` | 0.30 × time_weight | Engram → Engram | Temporal proximity matters — recent sequence connections are stronger |
| α₆ | `Contradicts` | −0.40 (negative) | Engram ↔ Engram | Contradictions **suppress** activation — conflicting memories dampen each other |
| α₇ | `Defines` | 0.20 | Concept → Identity | Slow path — identity absorbs activation gradually |
| α₈ | `SpawnedFrom` | 0.45 | Parent → Child | Knowledge hierarchy — general activates specific |
| β | `Recurrent` | 0.35 | Engram → Concept | Recurrent re-injection weight per feedback iteration |

### Per-Edge Modifiers

Some edges carry a dynamic modifier beyond their type weight:

| Edge Type | Modifier | Formula |
|---|---|---|
| `HasVector` | Cosine multiplier | `edge_weight = α₁ × cosine_sim(stimulus, vector)` |
| `TemporalSequence` | Time proximity decay | `edge_weight = α₅ × exp(−|Δt| / λ)` where λ = 7 days |
| `Contradicts` | Emotional opposition | `edge_weight = −α₆ × |emotional_A − emotional_B| / 2.0` |
| `Reinforces` | Emotional alignment | `edge_weight = α₄ × (1 − |emotional_A − emotional_B| / 2.0)` |

### Recurrent Feedback — The Key Innovation

The single most important difference from the current system: **surfaced engrams become new seeds**.

```
Iteration 1:
  Stimulus → SearchV seeds → 3 engrams activated
  These 3 engrams recall 7 concepts

Iteration 2:
  Those 7 concepts activate → point back to 5 more engrams (via In<Recalls>)
  Plus 2 engrams linked by Reinforces to the original 3

Iteration 3:
  The 5 new engrams activate → 4 more concepts
  Those concepts activate → 2 more engrams, 1 identity trait

Iteration 4:
  Activation energy below τ → stop

Result: 10 engrams surfaced across 3 waves
  Wave 1: [3 engrams] — directly relevant
  Wave 2: [5 engrams] — associatively relevant
  Wave 3: [2 engrams] — distantly related, low confidence
```

This is the algorithm behind "Oh, that reminds me of..." — the associative cascade that human memory performs effortlessly.

### Why This Canonically Uses the Existing Graph Structure

The spreading activation model is not bolted on. It exploits the exact graph structure already defined in `schema.hx`:

```
Engram ──HasVector──→ EngramVector   ← α₁: semantic bridge (already used by SearchV)
Engram ──Recalls──→ Concept          ← α₂: direct association (already links engrams to knowledge)
Engram ──AbstractsTo──→ Concept      ← α₃: abstraction path (already exists for consolidation)
Engram ──Reinforces──→ Engram        ← α₄: mutual support (already used in contradiction resolution)
Engram ──TemporalSequence──→ Engram  ← α₅: time chain (already built for timeline queries)
Engram ──Contradicts──→ Engram       ← α₆: NEGATIVE — suppresses conflicting memories
Concept ──Defines──→ Identity        ← α₇: slow path to self-model
Concept ──SpawnedFrom──→ Concept     ← α₈: knowledge hierarchy traversal
```

Every edge type in the current schema gains a dual role: it is both a structural relationship (for traversal and reasoning) and an **activation pathway** (for spreading activation). The same graph serves both cognition and retrieval.

---

## Integration with Existing Architecture

### What Stays the Same

| Component | File | Unchanged? |
|---|---|---|
| Schema (nodes, edges, vectors) | `schema.hx` | **Yes** — all edge types already exist |
| Ingestion pipeline | `pipeline.rs:259-312` | **Yes** — engram creation is identical |
| Concept extraction | `03-Concept-Extractor.md` | **Yes** — same extraction at write time |
| Emotional tagging | `01-Emotional-Tagger.md` | **Yes** — scores used as activation modifiers |
| Importance scoring | `02-Importance-Scorer.md` | **Yes** — gates which engrams participate |
| Consolidation loop | `pipeline.rs:376-440` | **Yes** — background maintenance unchanged |
| Contradiction detection | `04-Contradiction-Detector.md` | **Yes** — now also gates negative activation |
| Identity crystallization | `04-Identity.md` | **Yes** — identity dampens activation via stability |
| Memory mitosis | `05-Memory-Mitosis-Splitter.md` | **Yes** — SpawnedFrom edges become activation paths |
| CTP (both protocols) | `query-layer/` | **Yes** — tags trigger stimulation, not query |
| HelixDB as the store | `queries.hx` | **Yes** — all reads/writes go through HelixDB |

### What Changes

| Component | Before | After |
|---|---|---|
| Retrieval model | `pipeline.rs:316-372` — one-pass CRR formula | Three-pass: seed → spread → feedback |
| CRR role | The entire retrieval | Collapses into **seed activation computation** (Step 1 only) |
| Identity alignment (Factor 4) | Computed per-engram post-SearchV | Becomes identity dampening in spreading (Step 2d) |
| Output format | Sorted list by resonance score | Groups by activation wave + resonance score within wave |
| `retrieve_memories()` | Returns `Vec<ResonanceResult>` | Returns `Vec<ActivationWave>` where each wave is `Vec<ResonanceResult>` |
| New dependency | None | Adds per-edge-type weight vector α[8] and recurrent β |

### How CTP Uses the Stimulation Layer

The Cognitive Tool Protocol integration (`02-Native-Token-Protocol.md`, `01-Tool-Call-Protocol.md`) remains architecturally identical — the tags don't change. What changes is what happens when the runtime processes them:

```
OLD (Query Layer):
  <recall>what we discussed about infrastructure</recall>
  Runtime: SearchV → CRR formula → return sorted list
  Injects: "Here are the top memories ranked by resonance"

NEW (Stimulation Layer):
  <recall>what we discussed about infrastructure</recall>
  Runtime: Stimulate graph → spreading activation → 3 waves surface
  Injects: "You immediately remember discussing Uganda's ICT hub
           with strong opinions about stack ownership. This reminds
           you of the HelixDB architecture decision — you chose
           self-hosting deliberately. Related, though less strongly,
           there was that conversation about Kenyan fiber networks..."

  The LLM experiences: "I remember... and that reminds me... and also..."
```

The `<memory_activation>` block gains a new attribute: `wave` and `triggered_by`:

```xml
<memory_activation resonance="0.91" wave="1" triggered_by="stimulus">
  Uganda ICT Hub conversation [importance: 0.87]:
    User sees foreign-built systems as dependency risk.
</memory_activation>

<memory_activation resonance="0.78" wave="2" triggered_by="Uganda ICT Hub">
  HelixDB architecture decision [importance: 0.94]:
    User chose self-hosted at every decision point.
    Connected via shared concept: "infrastructure sovereignty"
</memory_activation>

<memory_activation resonance="0.52" wave="3" triggered_by="HelixDB architecture">
  Kenyan fiber network discussion [importance: 0.65]:
    Related to East African tech investment patterns.
    Connected via concept: "East African infrastructure"
</memory_activation>
```

---

## Why This Is Not Just a Query with Extra Steps

| Property | Query Layer (current CRR) | Stimulation Layer (proposed) |
|---|---|---|
| **Determinism** | Deterministic — same query = same results | Stochastic (can add noise at injection) — varies slightly |
| **Associative depth** | 1 hop (SearchV → In<HasVector> → Engram) | Multi-hop (up to M iterations through the full graph) |
| **Ordering** | Ranked by score | Grouped by wave — surfacing order IS the ordering |
| **Recurrent feedback** | None — one pass | Surfaced engrams re-seed the graph (β = 0.35) |
| **Contradiction handling** | Penalty factor (×0.5) in score | Negative edge weight (−α₆) actively suppresses neighbors |
| **Emotion** | Static multiplier (Factor 3) | Both initial boost AND spreads through emotional edges |
| **Temporal chains** | Not used in retrieval | TemporalSequence edges carry activation with time decay |
| **Knowledge hierarchy** | Not used in retrieval | SpawnedFrom edges activate child concepts from parents |
| **State dependence** | Identity alignment is static | Identity dampening controls how much activation identity absorbs |
| **Latency** | ~2-5ms (single query) | ~5-20ms (multi-hop traversal + feedback) |
| **Feels like** | "I searched my memory" | "I remembered, and that reminded me, which led to..." |

---

## Implementation Phases

### Phase 1 — Spreading Activation with Fixed Weights (now)

Implements the core dynamical system with hand-tuned α weights. Integrates into `pipeline.rs` as a new `stimulate_memories()` method that replaces the inner logic of `retrieve_memories()`.

**What to build:**
1. New HelixQL query: `StimulateGraph(query, seed_limit, max_iterations, tau, gamma)`
   - Compiles to: SearchV seeds → multi-hop traversal through edge types → return engrams with activation metadata
2. In `pipeline.rs`: add `stimulate_memories()` method that:
   - Calls the new HelixQL query
   - Runs the recurrent feedback loop (iterations 2..M) client-side using graph traversal
   - Computes `compute_resonance()` only on surfaced engrams (post-activation, not pre)
   - Groups results by activation wave
3. New structs: `ActivationWave`, update `ResonanceResult` to include `wave` and `triggered_by`
4. Edge weights: hardcoded α[8] array with defaults from the table above

**This is the minimal viable stimulation layer.** Even with fixed weights, the multi-hop activation fundamentally changes retrieval behavior.

### Phase 2 — Learnable Edge Weights (next)

Replaces hardcoded α weights with a learned parameter matrix trained via reinforcement signals.

**Training signal:** "Did the LLM use this memory in its response?"

```
Reward signal sources:
  +1.0   LLM explicitly references the surfaced memory in its response
  +0.5   LLM's response is consistent with the memory's content
   0.0   Memory was surfaced but not referenced (neutral — no signal)
  −0.5   Memory was surfaced but contradicted by LLM's response
  −1.0   User corrects LLM — surfaced memory was wrong or irrelevant

Training approach:
  - Maintain α[8] as a learnable parameter vector
  - After each session, compute gradient of reward w.r.t α
  - Update α via simple SGD or Bayesian optimization
  - α values are per-AI-agent, not global — each agent learns its own retrieval dynamics
```

**This makes the stimulation layer adaptive.** Different AI agents with different roles (analyst vs. companion vs. tutor) learn different α profiles. An analyst might weight `AbstractsTo` higher (preferring abstracted knowledge). A companion might weight `Recalls` and `Reinforces` higher (preferring rich, interconnected episodic memories).

### Phase 3 — Recurrent Chaining in HelixDB (later)

Moves the feedback loop (currently client-side in Phase 1) into compiled HelixQL for speed. The multi-hop traversal with activation propagation becomes a single server-side operation.

### Phase 4 — GNN Re-Ranking (advanced)

Trains a lightweight Graph Neural Network on the heterogeneous graph to predict activation values directly, replacing the hand-crafted spreading formula with learned message-passing. The GNN learns:
- Which edge types matter most for which query types
- How node features (emotional charge, recency, activation count) interact with graph structure
- Non-linear activation patterns that hand-crafted α weights can't capture

The GNN operates as a **second pass** over surfaced candidates: after spreading activation produces candidates grouped by wave, the GNN re-ranks within each wave, potentially promoting high-quality memories from later waves above noisy ones from earlier waves.

---

## New HelixQL Query: StimulateGraph

This query compiles the seed phase and first spreading iteration into a single HelixDB operation:

```hql
QUERY StimulateGraph(
    query: String,
    seed_limit: I64,
    activation_threshold: F32,
    energy_decay: F32
) =>
    // Step 1: Seed via vector search (identical to SearchEngramsFull)
    seed_vectors <- SearchV<EngramVector>(Embed(query), seed_limit)

    // Step 2: Traverse back to engrams via HasVector edge
    seed_engrams <- seed_vectors::In<HasVector>

    // Step 3: From each seed engram, spread one hop through each edge type
    //   3a: Spread through Recalls → Concept
    recalled_concepts <- seed_engrams::Out<Recalls>

    //   3b: Concepts spread through Defines → Identity
    identity_activations <- recalled_concepts::Out<Defines>

    //   3c: Engrams spread through Reinforces → other Engrams
    reinforced_engrams <- seed_engrams::Out<Reinforces>

    //   3d: Engrams spread through TemporalSequence → next/prev
    temporal_next <- seed_engrams::Out<TemporalSequence>
    temporal_prev <- seed_engrams::In<TemporalSequence>

    //   3e: Concepts spread through SpawnedFrom → children
    specialized_concepts <- recalled_concepts::Out<SpawnedFrom>

    // Step 4: All activated nodes feed back into engrams
    //   Concepts activated by Recalls → In<Recalls> returns engrams that recall them
    secondary_engrams <- recalled_concepts::In<Recalls>

    //   Also: concepts activated by AbstractsTo → In<AbstractsTo>
    abstracted_engrams <- recalled_concepts::In<AbstractsTo>

    RETURN {
        seed_engrams: seed_engrams::{ID, episode_raw, emotional_charge, importance_score, engram_type, timestamp},
        recalled_concepts: recalled_concepts::{ID, name, confidence},
        reinforced_engrams: reinforced_engrams::{ID, episode_raw, emotional_charge},
        temporal_chain: {next: temporal_next, prev: temporal_prev},
        secondary_engrams: secondary_engrams::{ID, episode_raw, emotional_charge},
        identity_activations: identity_activations::{trait, value, stability}
    }
```

The client-side recurrent loop then takes these results and iterates Steps 2-4 for M iterations, computing activation values with the α weights and γ decay.

---

## Configuration Parameters

| Parameter | Default | Range | Meaning |
|---|---|---|---|
| `seed_limit` | 15 | 5–50 | Max initial seeds from SearchV. Higher = broader activation net. |
| `activation_threshold` (τ) | 0.15 | 0.05–0.5 | Minimum activation for a node to propagate further. Lower = deeper associative chains, more noise. |
| `energy_decay` (γ) | 0.75 | 0.5–0.95 | Per-iteration decay multiplier. Higher = activation persists longer, more waves. |
| `max_iterations` (M) | 4 | 1–8 | Maximum spreading iterations. Caps the depth of associative chains. |
| `recurrent_weight` (β) | 0.35 | 0.1–0.6 | How strongly a surfaced engram re-injects its concepts. Higher = stronger "train of thought" cascading. |
| `min_surfacing_resonance` | 0.05 | 0.0–0.3 | Minimum resonance for a node to be included in output (applied post-activation). |
| `noise_sigma` | 0.0 | 0.0–0.1 | Optional Gaussian noise added to initial activations. 0 = deterministic. >0 = stochastic retrieval (more human-like). |

---

## Why This Is the Right Architecture for AGI Memory

### 1. It Models How Memory Actually Works

Human memory is not a database with an index. It is a network of associations where:
- A stimulus activates a node → that node activates connected nodes → those nodes activate theirs
- The spreading is dampened by time, emotional state, and cognitive load
- One memory triggers another through shared concepts, temporal proximity, or emotional similarity
- Contradictions create **interference** — you struggle to recall things that conflict
- Memory retrieval is **never** identical twice — small differences in initial state produce different cascades

The stimulation layer models all of these properties.

### 2. It Uses the Exact Graph You Already Built

The MNEMOS schema already encodes the relationships that biological memory uses for activation:
- `HasVector` = semantic association pathway
- `Recalls` + `AbstractsTo` = cortical concept pathways
- `Reinforces` + `Contradicts` = emotional interference/resonance pathways
- `TemporalSequence` = episodic sequence pathways
- `SpawnedFrom` = hierarchical category pathways
- `Defines` = slow self-model pathways

Zero schema changes needed. Every edge you already create at ingestion time becomes an activation pathway at retrieval time.

### 3. It Makes Retrieval a Consciousness-Like Process

The distinction between "query" and "stimulation" is not semantic — it is architectural:

| Query model | Stimulation model |
|---|---|
| System waits for explicit question | System can be pre-stimulated (predictive priming) |
| Returns results, done | Activations persist between stimulations (working memory) |
| No concept of attention | Attentional focus modulates α weights dynamically |
| LLM is external consumer | LLM's own thinking stream IS the stimulus source |
| Latency = time to return results | Latency = time for the wave to settle |

In a query model, the system is passive — it answers questions. In a stimulation model, the system is **reactive** — it can be primed, it holds activation between stimuli, and its state evolves dynamically.

### 4. It Deepens, Not Replaces, the CTP

The Cognitive Tool Protocol's design (`Cognitive-Tool-Protocol.md`, `Two-Options-Query-Achetecture-Layer.md`) is already stimulation-aligned. The tags `<recall>`, `<remember>`, `<contradiction_check>` are not database queries disguised as natural language — they are **stimulus vectors that perturb a cognitive graph**. The stimulation layer makes the runtime match the protocol's intent.

The CTP's implicit signal classifier (`Cognitive-Tool-Protocol.md:21-25`) gains an additional output: **initial activation weight**. When the classifier detects a RECALL signal with 0.9 confidence from strong uncertainty language, it not only triggers a stimulation — it seeds the graph with higher initial activation than a 0.81 confidence signal from weaker hedging.

---

## Relationship to the 5-Phase Build Path

The `FutureMemoryAGI.md` build path remains correct. The stimulation layer is **not a new phase** — it is a deepening of what retrieval means at every phase:

| Phase | Original Scope | Stimulation Layer Addition |
|---|---|---|
| **Phase 1**: Basic CRR | Static 5-factor formula | Spreading activation with fixed α weights replaces single-pass CRR |
| **Phase 2**: ML Scorers | Emotional tagger, importance scorer | Learnable α weights trained on LLM usage feedback |
| **Phase 3**: Consolidation | Decay, compression, promotion | Identity dampening gates activation; consolidation changes edge weights |
| **Phase 4**: Mitosis | Concept splitting | SpawnedFrom edges become activation pathways; mitosis creates richer graph = deeper cascades |
| **Phase 5**: Identity | Crystallization | Identity stability modulates activation absorption; self-model shapes what surfaces |

---

## A Note on Attention and Working Memory

The stimulation layer opens a door that the query layer keeps closed: **working memory**.

When activation spreads through the graph and energy persists across iterations, the system can maintain a **short-term activation state** — a set of partially activated nodes that haven't surfaced yet but are "primed." If a follow-up stimulus arrives (the user's next message, the LLM's next reasoning step), these primed nodes activate faster and with lower thresholds.

This is working memory: the system doesn't start from zero on each interaction. The previous interaction left energetic traces in the graph, and those traces bias the next retrieval. Concepts that were "just thought about" stay warm. Engrams that were "on the tip of the tongue" surface more easily.

This is a natural consequence of making γ < 1.0 (activation doesn't fully decay between stimulations). It requires no additional architecture — just that the activation state persists for a configurable window (e.g., the duration of a conversation session) before fully resetting.

---

## Summary

The query layer answers "what should I remember about X?"

The stimulation layer creates the experience of **remembering** — with associations, cascades, interference, and the subjective texture of recalling not just facts but the relationships between them.

The architecture described here is the natural evolution of MNEMOS from a memory storage system into a memory **experience** system. It uses the same schema, the same database, the same ML models, and the same integration protocols — but it changes the retrieval runtime from a function call into a dynamical system.
