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

## Recommended Implementation: Learnable Edge Weights (Adam + 9 params)

> **This is the recommended approach.** The stimulation layer with learnable weights personalizes retrieval to each user. The complexity over fixed weights is minimal (~20 lines of code for Adam update) but the benefit is dramatic — the system learns YOUR retrieval patterns.

### What to Build

1. **New `stimulate_memories()` method** in the pipeline:
   - Vector search seeds initial Engram nodes directly (no `::In<HasVector>` traversal)
   - Runs recurrent feedback loop (iterations 2..M) client-side using graph traversal
   - Computes resonance scoring on surfaced engrams (post-activation, not pre)
   - Groups results by activation wave
   - Records activation trace for learning

2. **New structs:**
   - `ActivationWave` — represents one wave of activation
   - `StimulationTrace` — records which edges contributed (for attribution)
   - Update `ResonanceResult` to include `wave` and `triggered_by`

3. **Learnable edge weights:** Adam optimizer on 9 parameters (~616 bytes total)

### Implementation

```rust
/// Stimulation layer configuration with learnable weights
pub struct StimulationConfig {
    pub seed_limit: usize,           // L: max initial seeds (default: 15)
    pub max_iterations: usize,       // M: max spreading iterations (default: 4)
    pub energy_decay: f64,           // γ: per-iteration decay (default: 0.75)
    pub surfacing_threshold: f64,    // τ: min activation to propagate (default: 0.15)
    pub learning_rate: f64,          // η: Adam learning rate (default: 0.01)
}

/// Result of one stimulation run
pub struct StimulationResult {
    pub waves: Vec<ActivationWave>,  // grouped by activation wave
    pub trace: StimulationTrace,     // for learning
}

pub struct ActivationWave {
    pub wave_number: usize,
    pub engrams: Vec<ResonanceResult>,
}

impl MemoryPipeline {
    /// Stimulate memories using spreading activation with learnable weights
    pub async fn stimulate_memories(
        &mut self,
        query_embedding: Vec<f32>,
        config: &StimulationConfig,
    ) -> Result<StimulationResult, Box<dyn std::error::Error>> {
        // Step 1: Seed phase — vector search returns Engram nodes DIRECTLY
        let seed_request = search_engrams_full(query_embedding, config.seed_limit as i64, 0.0);
        let seed_response = self.client.query(seed_request).send().await?;
        let seeds: Vec<EngramCandidate> = serde_json::from_value(
            seed_response["candidates"].clone()
        )?;

        // Initialize activation map
        let mut activation_map: HashMap<u64, f64> = HashMap::new();
        for seed in &seeds {
            let semantic_sim = 1.0 - seed.distance;
            let recency = compute_recency_weight(&seed.timestamp);
            let emotional_boost = 1.0 + seed.emotional_charge.abs();
            let contradiction_factor = if seed.contradiction_flag { 0.5 } else { 1.0 };
            activation_map.insert(seed.id, semantic_sim * recency * emotional_boost * contradiction_factor);
        }

        // Step 2-4: Spreading phase with recurrent feedback
        let mut waves: Vec<ActivationWave> = vec![];
        let mut trace = StimulationTrace::new();

        for iteration in 0..config.max_iterations {
            let mut new_activation: HashMap<u64, f64> = HashMap::new();

            for (node_id, activation) in &activation_map {
                if *activation < config.surfacing_threshold {
                    continue;
                }

                // Spread to neighbors via each edge type using LEARNABLE weights
                // (queries neighbors and applies α weights from self.edge_weights)
                // Records contributions to trace for later learning
                // ...
            }

            // Merge + decay
            for (node_id, act) in &activation_map {
                activation_map.insert(*node_id, (act + new_activation.get(node_id).unwrap_or(&0.0)) * config.energy_decay);
            }

            // Capture surfaced engrams for this wave
            let surfaced: Vec<ResonanceResult> = activation_map.iter()
                .filter(|(_, act)| **act > config.surfacing_threshold)
                .map(|(id, act)| {
                    // Build ResonanceResult from cached data
                    ResonanceResult { /* ... */ }
                })
                .collect();

            if !surfaced.is_empty() {
                waves.push(ActivationWave {
                    wave_number: iteration + 1,
                    engrams: surfaced,
                });
            }
        }

        // Store trace for learning (reward extracted later from LLM response)
        self.last_stimulation_trace = Some(trace.clone());

        Ok(StimulationResult { waves, trace })
    }

    /// Learn from feedback after LLM responds
    pub async fn learn_from_feedback(
        &mut self,
        reward: f64,  // +1.0 to -1.0
    ) -> Result<(), Box<dyn std::error::Error>> {
        let trace = match &self.last_stimulation_trace {
            Some(t) => t,
            None => return Ok(()),
        };

        // Compute attributions from trace
        let (attributions, recurrent_attr) = compute_attribution(trace, reward);

        // Adam update on edge weights
        self.edge_weights.adam_update(&attributions, recurrent_attr, reward);

        // Persist weights to disk
        self.save_weights().await?;

        Ok(())
    }
}
```

### Why Learnable Weights?

| Feature | Fixed Weights | Learnable Weights (Recommended) |
|---------|---------------|--------------------------------|
| Personalization | None | **Adapts to each user** |
| Complexity | Simpler | Slightly more (~20 lines Adam) |
| Memory cost | 72 bytes | **~616 bytes** (still tiny) |
| Compute cost | ~10μs | **~20μs** (still trivial) |
| Outcome | Static retrieval | **Personalized retrieval dynamics** |

The learnable weights approach is the best because it makes the system adapt to YOUR usage patterns. After 47 sessions, the system learns that YOU value concept-level associations and deep chaining — fixed weights can never do that.

---

## Full Implementation Details

For the complete learnable weights implementation (Adam optimizer, reward extraction, attribution engine), see:
- [08-Learnable-Edge-Weights.md](./08-Learnable-Edge-Weights.md)

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
- `Recalls` + `AbstractsTo` = cortical concept pathways
- `Reinforces` + `Contradicts` = emotional interference/resonance pathways
- `TemporalSequence` = episodic sequence pathways
- `SpawnedFrom` = hierarchical category pathways
- `Defines` = slow self-model pathways

Zero schema changes needed. Every edge you already create at ingestion time becomes an activation pathway at retrieval time. The embedding is stored directly on the Engram node, so vector search returns Engram nodes directly — no `HasVector` traversal needed.

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

---

## Implementation

For the actual Rust DSL implementation of queries mentioned in this doc, see:
- [Query Reference](../helixdb-embedded-queries.md)
- [Full Pipeline](../helixdb-embedded-pipeline.rs)
- [Retrieval Layer](../helixdb-embedded-retrieval.md)
- [Schema Definition](../helixdb-embedded-schema.md)

For learnable edge weights (the optional enhancement to this stimulation layer), see:
- [08-Learnable-Edge-Weights.md](./08-Learnable-Edge-Weights.md)
