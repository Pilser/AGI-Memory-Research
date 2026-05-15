# Learnable Edge Weights — Real-Time SGD on 9 Parameters

## What We're Actually Learning

The stimulation layer's behavior is governed by 9 scalar weights that control how much activation each edge type transmits:

```
α₁ : HasVector      — semantic bridge weight
α₂ : Recalls        — engram→concept association
α₃ : AbstractsTo    — engram→concept abstraction
α₄ : Reinforces     — engram↔engram mutual support
α₅ : TemporalSeq    — engram→engram time chain
α₆ : Contradicts    — engram↔engram suppression (negative)
α₇ : Defines        — concept→identity (slow path)
α₈ : SpawnedFrom    — concept parent→child hierarchy
β  : Recurrent      — re-injection weight per feedback iteration
```

That's it. Not a neural network. Not a matrix. Nine floating-point numbers.

---

## The Learning Loop

### Step 1: Capture the Reward Signal

After every retrieval, the LLM responds. We extract a scalar reward from how the surfaced memories were used:

```
┌─────────────────────────────────────────────────────┐
│                REWARD SIGNAL SOURCES                  │
│                                                       │
│  Explicit signals (high confidence):                  │
│    +1.0  LLM directly quotes/paraphrases the memory   │
│    −1.0  User corrects — memory was wrong             │
│                                                       │
│  Implicit signals (medium confidence):                │
│    +0.5  LLM response aligns with memory's content    │
│            (semantic similarity > 0.7)                │
│    +0.3  Memory's concepts appear in LLM's reasoning   │
│            (concept overlap > 0.5)                    │
│    −0.3  LLM contradicts the memory without comment   │
│            (emotional_charge opposite sign)           │
│                                                       │
│  Null signal:                                         │
│     0.0  Memory surfaced but not referenced            │
│            (neither positive nor negative)            │
│                                                       │
│  Composite reward:                                    │
│     R = Σ reward[e] × resonance[e] / Σ resonance[e]   │
│          weighted by how strongly each engram          │
│          contributed to the response                  │
└─────────────────────────────────────────────────────┘
```

### Step 2: Compute Per-Weight Attribution

We need to know how much each α weight **contributed** to each memory surfacing. The activation trace gives us this:

```python
def compute_attribution(stimulation_trace, surfaced_engrams, reward):
    """
    stimulation_trace: record of which edges were traversed,
                       with their contributions to each surfacing
    """
    attributions = {i: 0.0 for i in range(8)}  # α₀..α₇
    recurrent_attr = 0.0  # β

    for engram in surfaced_engrams:
        # How did this engram get activated?
        for edge_contribution in engram.activation_path:
            edge_type_idx = edge_contribution.edge_type
            contribution = edge_contribution.transferred_activation

            # The edge's contribution to THIS engram's surfacing
            attributions[edge_type_idx] += contribution * reward * engram.resonance

        if engram.surfaced_via_recurrent:
            recurrent_attr += engram.activation_path[-1].contribution * reward * engram.resonance

    # Normalize so attributions sum to 1
    total = sum(attributions.values()) + abs(recurrent_attr)
    if total > 0:
        attributions = {k: v/total for k, v in attributions.items()}
        recurrent_attr /= total

    return attributions, recurrent_attr
```

The key insight: we already know **exactly** which edges contributed to each activation because the spreading algorithm records the full activation trace per node. Attribution doesn't require approximation — it's directly observed from the stimulation run.

### Step 3: Gradient Update

```
For each α_i:
    α_i ← α_i + η × attribution_i × reward × (1 if reward > 0 else −α_i restraint)

For β:
    β ← β + η × reaction_attr × reward × (1 if reward > 0 else −β restraint)

Where:
    η  = learning rate (default 0.01)
    restraint = how much to pull α toward zero on negative reward
                (prevents α from going negative for positive edge types)
```

Full update function:

```python
def sgd_update(alphas, beta, attributions, recurrent_attr, reward, 
               learning_rate=0.01, restraint=0.1):
    """
    Update α weights using stochastic gradient descent.
    Positive reward → push active α upward.
    Negative reward → pull active α downward.
    Zero reward → no update (don't learn from noise).
    """

    if reward == 0.0:
        return alphas, beta  # No signal, no update

    for i in range(len(alphas)):
        attr = attributions.get(i, 0.0)

        if attr < 0.001:
            continue  # This edge type wasn't involved — don't touch it

        # Gradient direction
        if reward > 0:
            # Positive reward: increase weight of contributing edges
            gradient = attr * reward
        else:
            # Negative reward: decrease weight, but resist going out of bounds
            gradient = attr * reward * (alphas[i] / (abs(alphas[i]) + 0.1))  # soften near zero

        alphas[i] += learning_rate * gradient

        # Clamp to valid range
        if i == 6:  # Contradicts — must stay negative
            alphas[i] = min(alphas[i], -0.05)   # max negative is slightly below zero
            alphas[i] = max(alphas[i], -0.95)   # min negative is near -1.0
        else:        # All other edges — must stay positive
            alphas[i] = max(alphas[i], 0.01)    # min positive is just above zero
            alphas[i] = min(alphas[i], 0.98)    # max positive is near 1.0

    # Update recurrent weight β
    if abs(recurrent_attr) > 0.001:
        if reward > 0:
            beta += learning_rate * recurrent_attr * reward
        else:
            beta += learning_rate * recurrent_attr * reward * (beta / (beta + 0.1))
        beta = max(beta, 0.05)
        beta = min(beta, 0.75)

    return alphas, beta
```

### Step 4: Persist and Decay

Weights persist across sessions. A small regularization decay prevents overfitting to recent noise:

```python
# After each session, apply tiny L2 decay toward defaults
defaults = [0.85, 0.70, 0.50, 0.60, 0.30, -0.40, 0.20, 0.45]
for i in range(len(alphas)):
    alphas[i] = alphas[i] * (1 - 1e-4) + defaults[i] * 1e-4  # 0.01% toward default per session
beta = beta * (1 - 1e-4) + 0.35 * 1e-4
```

This means: if a user stops providing feedback for a long time, the weights slowly drift back to sensible defaults. The system doesn't permanently lock into a bad configuration from a few noisy sessions.

---

## The Full Session: Learning in Action

```
Session #1 (fresh agent, default weights):
  
  User: "What database should we use?"
  
  Stimulation trace:
    α₁ (HasVector)     → activated 3 engrams from seed   [default: 0.85]
    α₂ (Recalls)       → activated 5 concepts            [default: 0.70]
    β  (Recurrent)     → 3 more engrams in wave 2        [default: 0.35]
    α₇ (Defines)       → activated 1 identity trait      [default: 0.20]
  
  Surfaced: 8 engrams about databases, Postgres, MongoDB
  
  LLM: "Based on our history, you prefer Postgres for reliability."
  → Reward: +1.0 (directly referenced the memory)
  
  Attribution: α₁=0.4, α₂=0.35, β=0.15, α₇=0.10
  Update:
    α₁: 0.85 + 0.01 × 0.4 × 1.0 = 0.854
    α₂: 0.70 + 0.01 × 0.35 × 1.0 = 0.7035
    β:  0.35 + 0.01 × 0.15 × 1.0 = 0.3515
    α₇: 0.20 + 0.01 × 0.10 × 1.0 = 0.201
  (Small changes — one session doesn't reshape retrieval)
  
  ---
  
Session #47 (after 46 sessions of reinforcement):

  User: "What database should we use?"
  
  Same stimulation, but now:
    α₁ = 0.91  (user consistently values semantic relevance)
    α₂ = 0.82  (user values concept-level associations)
    β  = 0.41  (moderate recurrent chaining — user likes trains of thought)
    α₇ = 0.28  (identity alignment matters more than default)
  
  Surfaced: same 8 engrams, but ORDER and RESONANCE differ:
    - Engrams about Postgres now rank higher (semantic pathway stronger)
    - Identity trait "prefers reliability over novelty" boosts alignment
    - Recurrent chain: databases → Postgres → specific migration memories
  
  LLM: "You've consistently chosen Postgres. The MongoDB migration
        experience reinforced this — you value reliability over
        flexibility. Your last decision was about replication
        architecture, which reminds me..."
  → Reward: +1.0
  
  Same query, different retrieval behavior. Personalized.
```

---

## Integration with pipeline.rs

The existing `retrieve_memories()` method gains a learning closure:

```rust
struct MemoryPipeline {
    client: HelixDB,
    importance_scorer: ImportanceScorer,
    emotional_tagger: EmotionalTagger,
    concept_extractor: ConceptExtractor,

    // NEW: learnable edge weights
    edge_weights: EdgeWeights,

    // NEW: learning state
    last_stimulation_trace: Option<StimulationTrace>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EdgeWeights {
    has_vector:     f32,  // α₁
    recalls:        f32,  // α₂
    abstracts_to:   f32,  // α₃
    reinforces:     f32,  // α₄
    temporal_seq:   f32,  // α₅
    contradicts:    f32,  // α₆ (negative)
    defines:        f32,  // α₇
    spawned_from:   f32,  // α₈
    recurrent:      f32,  // β
}

#[derive(Debug, Clone)]
struct StimulationTrace {
    /// For each surfaced engram, which edges contributed activation
    activation_paths: Vec<ActivationPath>,
}

#[derive(Debug, Clone)]
struct ActivationPath {
    engram_id: String,
    resonance: f32,
    /// Which edge types contributed, and how much activation each transferred
    edge_contributions: HashMap<EdgeType, f32>,
    surfaced_via_recurrent: bool,
}

impl MemoryPipeline {
    async fn stimulate_memories(
        &mut self,
        query: &str,
        limit: usize,
    ) -> Result<Vec<ActivationWave>, Box<dyn std::error::Error>> {
        // ... existing stimulation logic, using self.edge_weights ...

        // Store trace for later learning
        self.last_stimulation_trace = Some(trace);

        Ok(waves)
    }

    async fn learn_from_feedback(
        &mut self,
        reward: f32,  // +1.0 to -1.0 from LLM interaction
    ) -> Result<(), Box<dyn std::error::Error>> {
        let trace = match &self.last_stimulation_trace {
            Some(t) => t,
            None => return Ok(()),  // Nothing to learn from
        };

        // Compute attributions from trace
        let (attributions, recurrent_attr) = compute_attribution(trace, reward);

        // SGD update
        self.edge_weights = sgd_update(
            &self.edge_weights,
            attributions,
            recurrent_attr,
            reward,
            0.01,   // learning_rate
            0.1,    // restraint
        );

        // Persist weights to disk
        self.save_weights().await?;

        Ok(())
    }
}
```

---

## Can the 9 Parameters Become Smarter?

The question "can the parameters themselves be updated to become smarter" has two legitimate interpretations:

### Interpretation A: Better Optimization (the straightforward answer)

Replace vanilla SGD with Adam — an adaptive optimizer that adjusts learning rates per-parameter:

```
Instead of:
    α ← α + η × gradient

Use Adam:
    m_t = β₁ × m_{t-1} + (1 − β₁) × gradient     ← momentum
    v_t = β₂ × v_{t-1} + (1 − β₂) × gradient²     ← velocity
    α ← α + η × m_t / (√v_t + ε)                  ← adaptive step

Default: β₁ = 0.9, β₂ = 0.999, ε = 1e-8, η = 0.01
```

**What this buys you:**
- Parameters that update frequently get smaller steps (don't overreact)
- Parameters that rarely update get larger steps (catch up faster)
- Momentum smooths out noise from individual sessions

Adam on 9 parameters is computationally free. It's literally:

```python
# Adam state: 18 floats (m and v for each of 9 params)
m = [0.0] * 9
v = [0.0] * 9
beta1 = 0.9
beta2 = 0.999
eps = 1e-8
t = 0

def adam_update(alphas, gradients):
    for i in range(9):
        t += 1
        m[i] = beta1 * m[i] + (1 - beta1) * gradients[i]
        v[i] = beta2 * v[i] + (1 - beta2) * gradients[i]**2
        m_hat = m[i] / (1 - beta1**t)
        v_hat = v[i] / (1 - beta2**t)
        alphas[i] += 0.01 * m_hat / (sqrt(v_hat) + eps)
```

### Interpretation B: Context-Dependent Weights (the interesting answer)

The current design uses **global α weights** — same for all queries. But what if the optimal α profile depends on **what kind of query is being asked**?

A factual query ("What database did we choose?") should weight `Recalls` and `HasVector` higher — direct associations matter most.

A reasoning query ("Should we migrate?") should weight `AbstractsTo`, `TemporalSequence`, and `β` higher — the system needs abstracted knowledge, temporal context, and deep associative chains.

A contradiction query ("Does this conflict with our decision?") should weight `Contradicts` higher — suppressing conflicted memories matters more than surfacing new ones.

**The solution: a context vector that modulates α weights per query.**

```python
# Instead of: global α = [α₁, α₂, ..., α₈]
# Use:        α(query) = α_base + W × context_vector(query)

context_vector(query):
    # A 4-dimensional vector computed at stimulation time:
    [
        query_emotional_tone,        # -1.0 to 1.0 (from quick sentiment model)
        query_abstractness,          # 0.0 (concrete fact) to 1.0 (abstract reasoning)
        query_contradiction_risk,    # 0.0 to 1.0 (how likely this conflicts)
        query_temporal_scope,        # 0.0 (now) to 1.0 (historical)
    ]

W: 9×4 matrix (36 parameters total)
α_base: 9 parameters (default weights)
```

Now the system learns **36 parameters** (W matrix) instead of 9. Still tiny. Still SGD-learnable in realtime. Still runs on edge.

```python
def compute_context_weights(alphas_base, W, query):
    ctx = context_vector(query)  # [f32; 4]

    # Modulate base weights by context
    alphas_contextualized = alphas_base.copy()
    for i in range(9):
        modulation = dot_product(W[i], ctx)  # scalar
        alphas_contextualized[i] += modulation
        alphas_contextualized[i] = clamp(alphas_contextualized[i], bounds[i])

    return alphas_contextualized
```

**What this enables:**

| Query Type | ctx_vector | Learned effect |
|---|---|---|
| "What DB did we pick?" | [0.1, 0.05, 0.0, 0.1] | α₁↑, α₂↑ — direct recall, no chaining needed |
| "Should we migrate off Postgres?" | [−0.2, 0.7, 0.5, 0.3] | α₃↑, α₅↑, α₆↓(stronger negative) — abstract, temporal, contradiction-aware |
| "Tell me everything about our DB history" | [0.3, 0.4, 0.0, 0.8] | β↑↑, α₂↑ — deep recurrent chaining, concept associations |
| "I think managed cloud is better now" | [0.1, 0.2, 0.9, 0.6] | α₆↓↓↓ — strongly activate contradiction pathway |

### Interpretation C: Multi-Armed Bandit Over Optimizers (meta-learning)

The system can maintain multiple optimization strategies and dynamically select the best one:

```python
optimizers = {
    "SGD":        lambda g: g * 0.01,
    "Adam":       lambda g: adam_step(g),
    "RMSprop":    lambda g: rmsprop_step(g),
    "conservative": lambda g: g * 0.001,  # tiny steps — use when unsure
    "aggressive":   lambda g: g * 0.05,   # big steps — use when feedback is clear
}

# Track performance of each optimizer
optimizer_rewards = {name: [] for name in optimizers}

def select_optimizer():
    # UCB (Upper Confidence Bound) bandit selection
    for name in optimizers:
        if len(optimizer_rewards[name]) == 0:
            return name  # Try untested optimizers first

    scores = {}
    for name in optimizers:
        mean_reward = mean(optimizer_rewards[name])
        uncertainty = sqrt(2 * log(total_updates) / len(optimizer_rewards[name]))
        scores[name] = mean_reward + uncertainty

    return max(scores, key=scores.get)
```

The system discovers: "When the reward signal is noisy (user gave ambiguous feedback), conservative steps work better. When the signal is clear (user explicitly corrected me), aggressive steps are safe." This is automatic — the bandit learns it from data.

### Interpretation D: Distribution Shift Detection (knowing when to re-learn)

A user's preferences drift over time. What worked 100 sessions ago may not work now. The system needs to detect this:

```python
def detect_shift(recent_rewards, window=20, threshold=0.3):
    """
    Compare recent rewards to historical average.
    If recent is significantly worse, the world changed.
    """
    historical_mean = mean(all_rewards[:-window])
    recent_mean = mean(recent_rewards[-window:])

    drift = historical_mean - recent_mean

    if drift > threshold:
        # Boost learning rate — we need to re-adapt
        return True, min(0.05, base_lr * (1 + drift * 2))
    else:
        return False, base_lr
```

When drift is detected, the learning rate increases temporarily, and the system re-adapts to the new pattern. This prevents the weights from becoming stale without manual intervention.

---

## The Full Learning Architecture

```
┌──────────────────────────────────────────────────────────────┐
│                   LEARNING SUBSYSTEM                          │
│                                                                │
│  ┌─────────────────┐    ┌──────────────┐    ┌──────────────┐ │
│  │ Reward Extractor │───▶│ Attribution  │───▶│  Optimizer   │ │
│  │                  │    │  Engine      │    │  (Adam)      │ │
│  │ +1.0 reference   │    │              │    │              │ │
│  │ +0.5 alignment   │    │ Scans trace  │    │ Adaptive LR  │ │
│  │ −1.0 correction  │    │ Per-edge     │    │ Momentum     │ │
│  │  0.0 no signal   │    │ attribution  │    │ bounds       │ │
│  └─────────────────┘    └──────┬───────┘    └──────┬───────┘ │
│                                 │                   │         │
│                          ┌──────▼───────────────────▼───┐     │
│                          │       Parameter Store         │     │
│                          │                                │     │
│                          │  α_base[9]  +  W[9×4]        │     │
│                          │  (defaults)   (context mod)   │     │
│                          │                                │     │
│                          │  Adam state: m[9], v[9]       │     │
│                          │  Reward history: last 100     │     │
│                          │  Shift detector               │     │
│                          │  LR scheduler                 │     │
│                          └──────────────┬────────────────┘     │
│                                          │                      │
│  ┌───────────────────────────────────────▼───────────────────┐ │
│  │                 Stimulation Engine                        │ │
│  │                                                           │ │
│  │  1. Receive query → compute context_vector                │ │
│  │  2. α(query) = α_base + W × ctx      ← contextual weights │ │
│  │  3. Run spreading activation with α(query), β             │ │
│  │  4. Return surfaced engrams + activation trace             │ │
│  └───────────────────────────────────────────────────────────┘ │
│                                                                │
│  Total learnable parameters: 45 (9 base + 36 context matrix)   │
│  Total optimizer state: 18 floats (Adam m + v for 9 params)    │
│  Total memory: < 1KB                                           │
│  Training per session: ~20 microseconds (45 multiply-adds)      │
└──────────────────────────────────────────────────────────────┘
```

---

## On-Device Learning Profile

| Resource | Cost |
|---|---|
| Parameters to store | 45 × f32 = 180 bytes |
| Optimizer state (Adam) | 18 × f32 = 72 bytes |
| Reward history ring buffer | 100 × f32 = 400 bytes |
| Total memory | < 1KB |
| Attribution computation | O(surfaced_engrams × avg_edges_per_path) ≈ 50-200 operations |
| Gradient computation | 9 multiply-adds (basic) or 45 (context-aware) |
| Disk I/O (save weights) | Write 180 bytes to JSON file after each session |
| Does it need a GPU? | No |
| Does it need offline batch training? | No |
| Does it share data with a server? | No |

---

## Learner Sophistication Tiers

| Tier | Parameters | Optimizer | Context-Aware | Shift Detection | When to Use |
|---|---|---|---|---|---|
| **Tier 0** | 9 (global α) | SGD | No | No | Phase 1 — get learning working |
| **Tier 1** | 9 (global α) | Adam | No | No | Phase 1.1 — better convergence |
| **Tier 2** | 45 (α + W matrix) | Adam | Yes (4-dim context) | No | Phase 2 — contextual retrieval |
| **Tier 3** | 45 + bandit state | Adam with UCB optimizer selection | Yes | Yes | Phase 3 — fully adaptive |

All four tiers run on edge, in realtime, from user feedback alone. Tier 3 adds maybe 100 bytes of bandit state and a few hundred extra operations per session. The complexity jump from Tier 0 to Tier 3 is negligible in compute but significant in behavioral quality.

---

## Why This Works

The problem has friendly properties that make realtime SGD viable:

1. **Low dimension** — 9-45 parameters means fast convergence, no overfitting, no gradient noise
2. **Direct attribution** — we know exactly which edges contributed to each surfacing (the trace records it), so the gradient is zero-noise
3. **Bounded reward** — reward is always in [−1, +1], no gradient explosion
4. **Sparse updates** — only active edges get updated; inactive ones stay put (implicit regularization)
5. **Natural curriculum** — the user's conversations naturally progress from simple to complex topics as the relationship deepens; the learning problem gets harder gradually and naturally
6. **Personalized** — each AI agent learns retrieval dynamics specific to its user, its role, its domain. No two agents converge to the same weights

The system doesn't need to be "trained" in the ML sense. It learns continuously, from every interaction, getting slightly better at remembering what matters to you with every conversation.
