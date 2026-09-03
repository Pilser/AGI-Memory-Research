# HelixDB Embedded Retrieval — MNEMOS Reference

> How Cognitive Resonance Retrieval (CRR) and the Stimulation Layer work with the new SDK's traversal and search operations

---

## Overview

The old approach used separate HelixQL queries for retrieval, with vector search (`SearchV`) and graph traversals (`::Out<Recalls>`, `::In<HasVector>`) defined in `queries.hx`. The new SDK provides these as **Rust DSL operations** that can be composed dynamically in `#[query]` functions.

This document covers:
1. Vector search via the Rust DSL
2. Graph traversal patterns for retrieval
3. The CRR formula implementation
4. The Stimulation Layer (spreading activation)
5. Output formatting for CTP injection

---

## Vector Search via Rust DSL

### Basic Vector Search

```rust
use helix_db::dsl::prelude::*;

#[query]
fn search_engrams_by_vector(
    query_vector: Vec<f32>,
    limit: i64,
) -> ReadBatch {
    read_batch()
        .var_as("results", g()
            .vector_search_with(
                "EngramVector",      // label to search
                "embedding",          // property containing vectors
                query_vector,         // the query embedding
                limit,                // max results
                None,                 // no extra options
            )
            .value_map(Some(vec!["$id", "$distance", "content"])))
        .returning(["results"])
}
```

**Key points:**
- `$distance` is available on the hit stream — project it before traversing away
- `vector_search_with` requires an active vector index on the label+property
- The search returns approximate nearest neighbors (ANN) by default
- Distance metric is defined by the vector index (cosine, euclidean, manhattan)

### Vector Search (Direct — No Traversal Needed)

```rust
#[query]
fn search_engrams_full(
    query_embedding: Vec<f32>,
    limit: i64,
    min_importance: f64,
) -> ReadBatch {
    read_batch()
        .var_as("candidates", g()
            // Step 1: Vector search returns Engram nodes DIRECTLY
            .vector_search_with("Engram", "embedding", query_embedding, limit, None)
            // Step 2: Filter by importance and contradiction flag (no traversal needed!)
            .where_(Predicate::gte("importance_score", min_importance))
            .where_(Predicate::eq("contradiction_flag", false))
            // Step 3: Project needed fields
            .value_map(Some(vec![
                "$id", "episode_raw", "emotional_charge",
                "importance_score", "engram_type", "timestamp"
            ])))
        .returning(["candidates"])
}
```

**Key simplification:** No `::In<HasVector>` traversal. Vector search returns `Engram` nodes directly.

### Optional Enhancement: Concept-Level Search (Pre-Filter)

> **Optional enhancement** — search concepts first as a pre-filter, then drill down to engrams.
> This is FASTER for broad queries but gives coarser results than direct engram search.

```rust
#[query]
fn search_concepts_then_engrams(
    query_embedding: Vec<f32>,
    concept_limit: i64,
    engram_limit: i64,
    min_importance: f64,
) -> ReadBatch {
    read_batch()
        .var_as("candidates", g()
            // Step 1: Search concepts by centroid embedding
            .vector_search_with("Concept", "embedding", query_embedding, concept_limit, None)
            // Step 2: Get engrams recalling these concepts
            .in_(Some("Recalls"))
            // Step 3: Filter by importance and contradiction flag
            .where_(Predicate::gte("importance_score", min_importance))
            .where_(Predicate::eq("contradiction_flag", false))
            // Step 4: Project needed fields
            .value_map(Some(vec![
                "$id", "episode_raw", "emotional_charge",
                "importance_score", "engram_type", "timestamp",
            ])))
        .returning(["candidates"])
}
```

**When to use:**
- **Direct engram search** (`search_engrams_full`): Use when you need precise per-engram resonance scoring
- **Concept pre-filter** (`search_concepts_then_engrams`): Use for broad queries where you want to narrow down the search space first

**No degradation:** Both paths work. Concept pre-filter is just an optimization — you still apply full CRR scoring to the resulting engrams.

---

## Graph Traversal Patterns

### Traversal Operations

| Operation | Rust DSL | Result |
|-----------|-----------|--------|
| Outgoing edges | `.out(Some("Recalls"))` | Destination nodes |
| Incoming edges | `.in_(Some("Recalls"))` | Source nodes |
| Both directions | `.both(Some("Recalls"))` | Adjacent nodes either way |
| Outgoing edges (edges) | `.out_e(Some("Recalls"))` | The edges themselves |
| Incoming edges (edges) | `.in_e(Some("Recalls"))` | The edges themselves |

### Multi-Hop Traversals

```rust
// Engram → Concept → Identity (cognitive chain)
g().n(NodeRef::id(engram_id))
    .out(Some("Recalls"))      // Engram → Concept
    .out(Some("Defines"))      // Concept → Identity
    .value_map(Some(vec!["$id", "trait", "value", "stability"]))

// Vector → Engram → Concepts (search + expand)
g().vector_search_with("EngramVector", "embedding", query_vec, limit, None)
    .in_(Some("HasVector"))    // Vector → Engram
    .out(Some("Recalls"))      // Engram → Concept
    .value_map(Some(vec!["$id", "name", "confidence"]))
```

### Source Predicates (Index Push-Down)

For efficient lookups, use source predicates that leverage indexes:

```rust
// Filter at the source (uses index if available)
g().n_with_label_where("Engram", SourcePredicate::eq("engram_type", "episodic"))

// Filter mid-traversal (post-source filtering)
g().n_with_label("Engram")
    .where_(Predicate::eq("contradiction_flag", false))

// Combined: source predicate + additional filter
g().n_with_label_where("Engram", SourcePredicate::eq("engram_type", "episodic"))
    .where_(Predicate::gte("importance_score", 0.3))
```

**Rule:** Source predicates are eligible for index push-down. General `.where()` filters the current stream after its source.

---

## Cognitive Resonance Retrieval (CRR)

### The Formula

```
resonance(query, engram) =
    semantic_similarity(query_embedding, engram_embedding)   ← Factor 1
  × recency_weight(engram.timestamp, now)                    ← Factor 2
  × (1 + |engram.emotional_charge|)                          ← Factor 3
  × identity_alignment(engram, identity_graph)               ← Factor 4
  × (1 − engram.contradiction_flag × 0.5)                    ← Factor 5
```

### Implementation in Rust

```rust
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct EngramCandidate {
    #[serde(rename = "$id")]
    id: u64,
    episode_raw: String,
    emotional_charge: f64,
    importance_score: f64,
    engram_type: String,
    timestamp: String,
    #[serde(rename = "$distance")]
    distance: f64,  // cosine distance from vector search
}

#[derive(Debug, Deserialize)]
struct IdentityTrait {
    #[serde(rename = "$id")]
    id: u64,
    trait_name: String,
    value: f64,
    stability: f64,
}

#[derive(Debug)]
struct ResonanceResult {
    engram_id: u64,
    resonance_score: f64,
    episode_raw: String,
    emotional_charge: f64,
    importance_score: f64,
    identity_alignment: f64,
    semantic_sim: f64,
    recency_weight: f64,
}

fn compute_resonance(
    candidates: &[EngramCandidate],
    identities: &[IdentityTrait],
    now_ts: f64,
    semantic_sims: &[f64],  // 1.0 - cosine_distance for each candidate
) -> Vec<ResonanceResult> {
    candidates.iter().enumerate().map(|(i, c)| {
        // Factor 1: semantic similarity (from vector search)
        let semantic = semantic_sims.get(i).copied().unwrap_or(0.5);

        // Factor 2: recency — exponential decay over time
        let engram_ts = parse_timestamp(&c.timestamp);
        let days_elapsed = (now_ts - engram_ts) / 86400.0;
        let recency = (-0.01_f64 * days_elapsed).exp().max(0.01);

        // Factor 3: emotional charge amplification
        let emotional_boost = 1.0 + c.emotional_charge.abs();

        // Factor 4: identity alignment
        let identity_align = compute_identity_alignment(&c.episode_raw, identities);

        // Factor 5: contradiction penalty
        // (already filtered out in query, so 1.0 here)
        let contradiction_factor = 1.0;

        let resonance = semantic
            * recency
            * emotional_boost
            * identity_align
            * contradiction_factor;

        ResonanceResult {
            engram_id: c.id,
            resonance_score: resonance,
            episode_raw: c.episode_raw.clone(),
            emotional_charge: c.emotional_charge,
            importance_score: c.importance_score,
            identity_alignment: identity_align,
            semantic_sim: semantic,
            recency_weight: recency,
        }
    }).collect()
}

fn compute_identity_alignment(
    text: &str,
    identities: &[IdentityTrait],
) -> f64 {
    // Compare text embedding against identity trait embeddings.
    // Simple version: return 1.0 (neutral) until identity graph is built.
    // Advanced: embed text, cosine-sim against each identity trait's embedding.
    if identities.is_empty() {
        return 1.0;
    }
    // TODO: implement embedding-based alignment
    1.0
}

fn parse_timestamp(ts: &str) -> f64 {
    // RFC 3339 → unix seconds
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.timestamp() as f64)
        .unwrap_or(0.0)
}
```

### Full Retrieval Pipeline

```rust
async fn retrieve_memories(
    client: &Client,
    query_embedding: Vec<f32>,
    identity_graph: &[IdentityTrait],
    limit: usize,
) -> Result<Vec<ResonanceResult>, Box<dyn std::error::Error>> {
    // Step 1: Vector search + traverse to engrams + filter
    let search_request = search_engrams_full(query_embedding.clone(), 20, 0.0);
    let search_response = client.query(search_request).send().await?;

    // Step 2: Deserialize candidates
    let candidates: Vec<EngramCandidate> = serde_json::from_value(
        search_response["candidates"].clone()
    )?;

    // Step 3: Compute semantic similarities (1.0 - distance)
    let semantic_sims: Vec<f64> = candidates.iter()
        .map(|c| 1.0 - c.distance)
        .collect();

    // Step 4: Compute resonance scores
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs_f64();

    let mut results = compute_resonance(&candidates, identity_graph, now, &semantic_sims);

    // Step 5: Sort by resonance (highest first)
    results.sort_by(|a, b| b.resonance_score.partial_cmp(&a.resonance_score).unwrap());

    // Step 6: Take top N
    results.truncate(limit);

    // Step 7: Activate recalled engrams (bump activation_count)
    for r in &results {
        if r.resonance_score > 0.3 {
            // Read current activation count, then increment
            // (requires get_engram + activate_engram calls)
        }
    }

    Ok(results)
}
```

---

## Stimulation Layer (Spreading Activation)

### Concept

The Stimulation Layer replaces single-pass CRR with a dynamical system where:
1. **Seed phase:** Query embedding seeds initial nodes via vector search
2. **Spreading phase:** Activation propagates through edges with per-type weights
3. **Recurrent feedback:** Activated engrams re-inject their concepts as secondary seeds
4. **Decay and surfacing:** Activation decays per timestep; nodes above threshold surface

### Edge Type Weights

| Index | Edge Type | Default α | Direction |
|-------|-----------|-----------|-----------|
| α₁ | `Recalls` | 0.70 | Engram → Concept |
| α₂ | `AbstractsTo` | 0.50 | Engram → Concept |
| α₃ | `Reinforces` | 0.60 | Engram ↔ Engram |
| α₄ | `TemporalSequence` | 0.30 × time_weight | Engram → Engram |
| α₅ | `Contradicts` | −0.40 (negative) | Engram ↔ Engram |
| α₆ | `Defines` | 0.20 | Concept → Identity |
| α₇ | `SpawnedFrom` | 0.45 | Parent → Child |
| β | `Recurrent` | 0.35 | Engram → Concept (feedback) |

**Note:** α₁ (HasVector bridge) is eliminated — it was only needed to spread activation from vector search results back to engrams. With direct-on-node storage, the seed phase activates Engram nodes directly.

### Optional Enhancement: Dynamic Edge Weights with Edge Embeddings

> **Optional enhancement** — makes the Stimulation Layer more biologically plausible.
> Instead of fixed α weights for all edges of the same type, each edge transmits activation based on its relationship strength.

**Current (fixed weights):**
```rust
// All Recalls edges transmit the same amount of activation
let activation_transfer = activation[source] × 0.70;  // α₁ = 0.70 for all
```

**Enhanced (dynamic weights):**
```rust
// Each Recalls edge transmits based on its relationship embedding
let relationship_strength = compute_relationship_strength(edge, concept_centroid);
let activation_transfer = activation[source] × relationship_strength × α_base;
```

**Computing relationship strength:**
```rust
fn compute_relationship_strength(
    edge_embedding: &[f32],
    concept_centroid: &[f32],
) -> f64 {
    // Cosine similarity between edge embedding and concept centroid
    // High similarity = strong relationship = more activation transfer
    cosine_similarity(edge_embedding, concept_centroid)
}
```

**Benefits (no degradation):**
- **Biologically plausible:** Stronger associations spread more activation (like human memory)
- **More nuanced retrieval:** "Central" engrams to a concept activate more than "tangential" ones
- **Adaptive:** Edge weights evolve as concept centroids are updated

**When to use:**
- Use fixed weights (α₁ = 0.70) for simplicity
- Use dynamic weights when you need more biologically plausible spreading dynamics

### Implementation with New SDK

The Stimulation Layer requires **multiple queries** because the new DSL doesn't have a single "spread activation" operation. Instead, you implement the spreading loop client-side:

```rust
use std::collections::HashMap;

#[derive(Debug, Clone)]
struct ActivationNode {
    node_id: u64,
    node_type: String,  // "Engram", "Concept", "Identity"
    activation: f64,
    wave: usize,
}

const ALPHA: [f64; 8] = [0.85, 0.70, 0.50, 0.60, 0.30, -0.40, 0.20, 0.45];
const BETA: f64 = 0.35;
const GAMMA: f64 = 0.75;      // energy decay per iteration
const TAU: f64 = 0.15;        // surfacing threshold
const MAX_ITERATIONS: usize = 4;
const SEED_LIMIT: i64 = 15;

async fn stimulate_memories(
    client: &Client,
    query_embedding: Vec<f32>,
) -> Result<Vec<Vec<ResonanceResult>>, Box<dyn std::error::Error>> {
    let mut waves: Vec<Vec<ResonanceResult>> = vec![];
    let mut activation_map: HashMap<u64, f64> = HashMap::new();

    // ── SEED PHASE ──
    // Vector search returns Engram nodes DIRECTLY (no ::In<HasVector> traversal needed)
    let seed_request = search_engrams_full(query_embedding.clone(), SEED_LIMIT, 0.0);
    let seed_response = self.client.query(seed_request).send().await?;
    let seeds: Vec<EngramCandidate> = serde_json::from_value(
        seed_response["candidates"].clone()
    )?;

    for seed in &seeds {
        let semantic_sim = 1.0 - seed.distance;
        let recency = compute_recency_weight(&seed.timestamp);
        let emotional_boost = 1.0 + seed.emotional_charge.abs();
        // No α₁ (HasVector bridge weight) needed — direct activation
        let initial_activation = semantic_sim * recency * emotional_boost;
        activation_map.insert(seed.id, initial_activation);
    }

    // ── SPREADING PHASE ──
    for iteration in 0..MAX_ITERATIONS {
        let mut new_activations: HashMap<u64, f64> = HashMap::new();

        for (node_id, activation) in &activation_map {
            if *activation < TAU {
                continue;
            }

            // Spread through Recalls (α₂ = 0.70)
            let recalls_request = read_batch()
                .var_as("concepts", g()
                    .n(NodeRef::id(*node_id))
                    .out(Some("Recalls"))
                    .value_map(Some(vec!["$id", "name", "confidence"])))
                .returning(["concepts"]);
            let recalls_response = client.query(recalls_request).send().await?;
            // Process: new_activations[concept] += activation * ALPHA[1]

            // Spread through Reinforces (α₄ = 0.60)
            let reinforces_request = read_batch()
                .var_as("reinforced", g()
                    .n(NodeRef::id(*node_id))
                    .out(Some("Reinforces"))
                    .value_map(Some(vec!["$id", "episode_raw", "emotional_charge"])))
                .returning(["reinforced"]);
            let reinforces_response = client.query(reinforces_request).send().await?;
            // Process: new_activations[engram] += activation * ALPHA[3]

            // Spread through Contradicts (α₆ = -0.40, negative)
            let contradicts_request = read_batch()
                .var_as("contradicted", g()
                    .n(NodeRef::id(*node_id))
                    .out(Some("Contradicts"))
                    .value_map(Some(vec!["$id", "episode_raw", "emotional_charge"])))
                .returning(["contradicted"]);
            let contradicts_response = client.query(contradicts_request).send().await?;
            // Process: new_activations[engram] += activation * ALPHA[5] (negative!)

            // Recurrent feedback: concepts re-inject (β = 0.35)
            // (concepts activated above re-inject their engrams)
        }

        // Merge + decay
        for (node_id, new_act) in &new_activations {
            let old_act = activation_map.get(node_id).copied().unwrap_or(0.0);
            activation_map.insert(*node_id, (old_act + new_act) * GAMMA);
        }

        // Capture surfaced engrams for this wave
        let surfaced: Vec<ResonanceResult> = activation_map.iter()
            .filter(|(_, act)| **act > TAU)
            .map(|(id, act)| {
                // Build ResonanceResult from cached data
                // (you'd need to cache node data during traversal)
                ResonanceResult {
                    engram_id: *id,
                    resonance_score: *act,
                    episode_raw: String::new(), // from cache
                    emotional_charge: 0.0,       // from cache
                    importance_score: 0.0,       // from cache
                    identity_alignment: 1.0,
                    semantic_sim: 0.0,
                    recency_weight: 0.0,
                }
            })
            .collect();

        if !surfaced.is_empty() {
            waves.push(surfaced);
        }
    }

    Ok(waves)
}

fn compute_recency_weight(timestamp: &str) -> f64 {
    let engram_ts = chrono::DateTime::parse_from_rfc3339(timestamp)
        .map(|dt| dt.timestamp() as f64)
        .unwrap_or(0.0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs_f64();
    let days_elapsed = (now - engram_ts) / 86400.0;
    (-0.01_f64 * days_elapsed).exp().max(0.01)
}
```

### Optimization: Batch Traversals

The naive approach above makes many individual queries. For production, batch traversals into fewer requests:

```rust
// Instead of querying each node individually, fetch all neighbors in one query
#[query]
fn get_neighbors_batch(node_ids: Vec<u64>) -> ReadBatch {
    read_batch()
        .var_as("outgoing_recalls", g()
            .n(NodeRef::ids(node_ids.clone()))
            .out(Some("Recalls"))
            .value_map(Some(vec!["$id", "name", "confidence"])))
        .var_as("outgoing_reinforces", g()
            .n(NodeRef::ids(node_ids.clone()))
            .out(Some("Reinforces"))
            .value_map(Some(vec!["$id", "episode_raw", "emotional_charge"])))
        .var_as("outgoing_contradicts", g()
            .n(NodeRef::ids(node_ids))
            .out(Some("Contradicts"))
            .value_map(Some(vec!["$id", "episode_raw", "emotional_charge"])))
        .returning(["outgoing_recalls", "outgoing_reinforces", "outgoing_contradicts"])
}
```

---

## Output Formatting for CTP Injection

### Single-Pass CRR Output

```rust
fn format_for_crr_injection(results: &[ResonanceResult]) -> String {
    let mut output = String::new();
    for r in results {
        output.push_str(&format!(
            "<memory_activation resonance=\"{:.2}\" source=\"explicit\">\n\
             \x20\x20{} [importance: {:.2}]:\n\
             \x20\x20\x20\x20{}\n\
             </memory_activation>\n",
            r.resonance_score,
            r.episode_raw.chars().take(60).collect::<String>(),
            r.importance_score,
            r.episode_raw.chars().take(200).collect::<String>()
        ));
    }
    output
}
```

### Stimulation Layer Output (Wave-Based)

```rust
fn format_for_stimulation_injection(waves: &[Vec<ResonanceResult>]) -> String {
    let mut output = String::new();
    for (wave_idx, wave) in waves.iter().enumerate() {
        let wave_num = wave_idx + 1;
        for r in wave {
            let triggered_by = if wave_num == 1 {
                "stimulus".to_string()
            } else {
                format!("wave-{}", wave_num - 1)
            };
            output.push_str(&format!(
                "<memory_activation resonance=\"{:.2}\" wave=\"{}\" triggered_by=\"{}\">\n\
                 \x20\x20{} [importance: {:.2}]:\n\
                 \x20\x20\x20\x20{}\n\
                 </memory_activation>\n",
                r.resonance_score,
                wave_num,
                triggered_by,
                r.episode_raw.chars().take(60).collect::<String>(),
                r.importance_score,
                r.episode_raw.chars().take(200).collect::<String>()
            ));
        }
    }
    output
}
```

---

## Performance Considerations

### Query Count

| Approach | Queries per Retrieval | Latency |
|----------|----------------------|---------|
| Old (single-pass CRR) | 3 (search + identity + activation) | ~6-15ms |
| New (single-pass CRR) | 2-3 (search + identity + activation) | ~4-10ms |
| Stimulation Layer (naive) | 3 + (nodes × iterations × edge_types) | ~50-200ms |
| Stimulation Layer (batched) | 3 + (iterations × edge_types) | ~15-40ms |

### Optimization Strategies

1. **Batch traversals** — fetch all neighbors of multiple nodes in one query
2. **Cache identity graph** — load once per session, not per retrieval
3. **Pre-compute semantic sims** — vector search returns `$distance`, convert once
4. **Limit iterations** — cap `MAX_ITERATIONS` at 4 for real-time use
5. **Early termination** — stop spreading when activation energy falls below threshold

---

## Migration Notes

### What Changed

| Old | New |
|-----|-----|
| `SearchV<EngramVector>(Embed(query), limit)::In<HasVector>` | `g().vector_search_with("Engram", "embedding", query_vec, limit, None)` |
| `::In<HasVector>` traversal | **Eliminated** — vector search returns Engram directly |
| `::Out<Recalls>` traversal | `.out(Some("Recalls"))` in Rust DSL |
| `Embed()` function (server-side) | Compute embedding client-side, pass as parameter |
| Separate `EngramVector` node + `HasVector` edge | **Eliminated** — embedding on Engram directly |
| α₁ (HasVector bridge weight: 0.85) in Stimulation Layer | **Eliminated** — seed phase activates Engrams directly |

### What Stayed the Same

- Vector search concept (ANN on embeddings)
- Graph traversal semantics (out, in, both)
- CRR formula (5-factor multiplicative)
- Stimulation Layer algorithm (seed → spread → feedback)
- Wave-based output for CTP injection

### Optional Enhancements (Add Without Degradation)

| Enhancement | What it adds | Algorithms improved |
|-------------|--------------|---------------------|
| **Concept centroid embedding** | `embedding` property on Concept | CRR (pre-filter), Mitosis (faster decisions), Stimulation Layer (new seed option) |
| **Edge embedding on Recalls** | `relationship_embedding` on edges | Stimulation Layer (dynamic weights), Mitosis (relationship-based clustering) |
| **Concept vector index** | Vector search on Concept.embedding | CRR (concept-level search), Mitosis (centroid comparison) |
| **Dynamic edge weights** | Per-edge activation transfer | Stimulation Layer (biologically plausible spreading) |

### What's New

- **Composable queries** — build search + traverse + filter dynamically
- **Type-safe traversals** — compile-time checking of batch types
- **Client-side embedding** — no server-side `Embed()` function needed
- **Flexible parameters** — pass any values as `#[query]` arguments

---

## Related Documentation

- [Integration Guide](./helixdb-embedded-integration.md) — SDK setup, client init, storage backends
- [Schema Definition](./helixdb-embedded-schema.md) — nodes, edges, indexes
- [Query Reference](./helixdb-embedded-queries.md) — all `#[query]` functions
- [Full Pipeline](./helixdb-embedded-pipeline.rs) — complete reference implementation
- [FutureMemoryAGI.md](./FutureMemoryAGI.md) — system overview and architecture
