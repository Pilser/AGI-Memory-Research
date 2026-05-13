// ============================================================
// MNEMOS — Cognitive Memory Operating System
// HelixQL Queries (queries.hx)
// ============================================================
//
// Each query includes a Rust example using helix_rs.
// Run with:
//   use helix_rs::{HelixDB, HelixDBClient};
//   let client = HelixDB::new(Some("http://localhost"), Some(6969), None);

// ============================================================
// 1. CREATE OPERATIONS
// ============================================================

// Create a new engram node
QUERY CreateEngram(
    episode_raw: String,
    emotional_charge: F32,
    importance_score: F32,
    decay_rate: F32,
    activation_count: U32,
    engram_type: String,
    compression_level: U8,
    contradiction_flag: Boolean
) =>
    engram <- AddN<Engram>({
        episode_raw: episode_raw,
        emotional_charge: emotional_charge,
        importance_score: importance_score,
        decay_rate: decay_rate,
        activation_count: activation_count,
        timestamp: NOW,
        engram_type: engram_type,
        compression_level: compression_level,
        contradiction_flag: contradiction_flag
    })
    RETURN engram

// RUST:
// let result = client.query("CreateEngram", &json!({
//     "episode_raw": "User discussed Uganda ICT Hub infrastructure...",
//     "emotional_charge": 0.7,
//     "importance_score": 0.8,
//     "decay_rate": 0.05,
//     "activation_count": 0,
//     "engram_type": "episodic",
//     "compression_level": 0,
//     "contradiction_flag": false
// })).await?;

// ──────────────────────────────────────

// Create a concept node
QUERY CreateConcept(
    name: String,
    confidence: F32
) =>
    concept <- AddN<Concept>({
        name: name,
        confidence: confidence,
        formation_date: NOW,
        source_count: 1
    })
    RETURN concept

// RUST:
// let result = client.query("CreateConcept", &json!({
//     "name": "Uganda ICT Infrastructure",
//     "confidence": 0.6
// })).await?;

// ──────────────────────────────────────

// Create an identity trait
QUERY CreateIdentity(
    trait: String,
    value: F32,
    stability: F32
) =>
    identity <- AddN<Identity>({
        trait: trait,
        value: value,
        stability: stability,
        last_updated: NOW
    })
    RETURN identity

// ──────────────────────────────────────

// Create a vector embedding from raw text (uses Embed function)
QUERY CreateEngramVectorText(
    content: String
) =>
    vector <- AddV<EngramVector>(Embed(content), {content: content})
    RETURN vector

// RUST:
// let result = client.query("CreateEngramVectorText", &json!({
//     "content": "User discussed Uganda ICT Hub infrastructure..."
// })).await?;

// ──────────────────────────────────────

// Create a vector embedding from pre-computed array
QUERY CreateEngramVector(
    vector: [F64],
    content: String
) =>
    vector_node <- AddV<EngramVector>(vector, {content: content})
    RETURN vector_node

// RUST:
// let result = client.query("CreateEngramVector", &json!({
//     "vector": [0.12, 0.34, 0.56, ...],  // 1536 f64 values
//     "content": "User discussed Uganda ICT Hub infrastructure..."
// })).await?;

// ──────────────────────────────────────

// Create edge: HasVector (connect engram to its embedding)
QUERY ConnectEngramVector(
    engram_id: ID,
    vector_id: ID
) =>
    edge <- AddE<HasVector>::From(engram_id)::To(vector_id)
    RETURN edge

// ──────────────────────────────────────

// Create edge: Recalls (engram → concept)
QUERY ConnectRecalls(
    engram_id: ID,
    concept_id: ID
) =>
    edge <- AddE<Recalls>::From(engram_id)::To(concept_id)
    RETURN edge

// ──────────────────────────────────────

// Create edge: Contradicts
QUERY ConnectContradiction(
    engram_a_id: ID,
    engram_b_id: ID
) =>
    edge <- AddE<Contradicts>::From(engram_a_id)::To(engram_b_id)
    RETURN edge

// ──────────────────────────────────────

// Create edge: Reinforces
QUERY ConnectReinforcement(
    engram_a_id: ID,
    engram_b_id: ID
) =>
    edge <- AddE<Reinforces>::From(engram_a_id)::To(engram_b_id)
    RETURN edge

// ──────────────────────────────────────

// Create edge: AbstractsTo
QUERY ConnectAbstraction(
    engram_id: ID,
    concept_id: ID
) =>
    edge <- AddE<AbstractsTo>::From(engram_id)::To(concept_id)
    RETURN edge

// ──────────────────────────────────────

// Create edge: Defines (concept → identity)
QUERY ConnectDefinition(
    concept_id: ID,
    identity_id: ID
) =>
    edge <- AddE<Defines>::From(concept_id)::To(identity_id)
    RETURN edge

// ──────────────────────────────────────

// Create edge: TemporalSequence
QUERY ConnectTemporal(
    earlier_id: ID,
    later_id: ID
) =>
    edge <- AddE<TemporalSequence>::From(earlier_id)::To(later_id)
    RETURN edge

// ──────────────────────────────────────

// Create edge: SpawnedFrom (parent → child, for mitosis)
QUERY ConnectSpawn(
    parent_id: ID,
    child_id: ID
) =>
    edge <- AddE<SpawnedFrom>::From(parent_id)::To(child_id)
    RETURN edge

// ──────────────────────────────────────

// Atomic: create engram + its vector + connect them
QUERY CreateFullEngram(
    episode_raw: String,
    emotional_charge: F32,
    importance_score: F32,
    decay_rate: F32,
    engram_type: String,
    compression_level: U8,
    contradiction_flag: Boolean
) =>
    engram <- AddN<Engram>({
        episode_raw: episode_raw,
        emotional_charge: emotional_charge,
        importance_score: importance_score,
        decay_rate: decay_rate,
        activation_count: 0,
        timestamp: NOW,
        engram_type: engram_type,
        compression_level: compression_level,
        contradiction_flag: contradiction_flag
    })

    vector <- AddV<EngramVector>(Embed(episode_raw), {content: episode_raw})
    edge <- AddE<HasVector>::From(engram)::To(vector)
    RETURN engram


// ============================================================
// 2. READ OPERATIONS
// ============================================================

// Get engram by ID
QUERY GetEngram(engram_id: ID) =>
    engram <- N<Engram>(engram_id)
    RETURN engram

// ──────────────────────────────────────

// Get engram with its vector
QUERY GetEngramWithVector(engram_id: ID) =>
    engram <- N<Engram>(engram_id)
    vector <- engram::Out<HasVector>::FIRST
    RETURN {
        engram: engram,
        vector: vector
    }

// ──────────────────────────────────────

// Get all engrams of a given type
QUERY GetEngramsByType(engram_type: String) =>
    engrams <- N<Engram>::WHERE(_::{engram_type}::EQ(engram_type))
    RETURN engrams

// ──────────────────────────────────────

// Get concept by name (requires INDEX on name)
QUERY GetConceptByName(name: String) =>
    concept <- N<Concept>({name: name})
    RETURN concept

// ──────────────────────────────────────

// Get concepts an engram recalls
QUERY GetRecalledConcepts(engram_id: ID) =>
    concepts <- N<Engram>(engram_id)::Out<Recalls>
    RETURN concepts

// ──────────────────────────────────────

// Get engrams that recall a concept
QUERY GetEngramsRecallingConcept(concept_id: ID) =>
    engrams <- N<Concept>(concept_id)::In<Recalls>
    RETURN engrams

// ──────────────────────────────────────

// Get contradictions for an engram (both directions)
QUERY GetContradictions(engram_id: ID) =>
    out <- N<Engram>(engram_id)::Out<Contradicts>
    in <- N<Engram>(engram_id)::In<Contradicts>
    RETURN {outgoing: out, incoming: in}

// ──────────────────────────────────────

// Get identity traits defined by concepts linked to an engram
QUERY GetIdentityAlignment(engram_id: ID) =>
    identity_traits <- N<Engram>(engram_id)
        ::Out<Recalls>
        ::Out<Defines>
    RETURN identity_traits

// ──────────────────────────────────────

// Get the abstraction chain: engram → concepts → identity
QUERY GetCognitiveChain(engram_id: ID) =>
    engram <- N<Engram>(engram_id)
    concepts <- engram::Out<Recalls>
    identities <- concepts::Out<Defines>
    RETURN {
        engram: engram::{episode_raw, engram_type, emotional_charge, importance_score},
        concepts: concepts::{name, confidence},
        identities: identities::{trait, value, stability}
    }


// ============================================================
// 3. VECTOR SEARCH (Cognitive Resonance Retrieval — Phase 1)
// ============================================================

// Simple vector similarity search on engram vectors
QUERY SearchEngramsByVector(vector: [F64], limit: I64) =>
    results <- SearchV<EngramVector>(vector, limit)
    RETURN results

// RUST:
// let results = client.query("SearchEngramsByVector", &json!({
//     "vector": query_embedding,   // [F64; 1536]
//     "limit": 20
// })).await?;

// ──────────────────────────────────────

// Text-based semantic search (uses Embed function)
QUERY SearchEngramsByText(query: String, limit: I64) =>
    results <- SearchV<EngramVector>(Embed(query), limit)
    RETURN results

// RUST:
// let results = client.query("SearchEngramsByText", &json!({
//     "query": "infrastructure development in Africa",
//     "limit": 20
// })).await?;

// ──────────────────────────────────────

// Vector search + retrieve connected engram data (single query)
QUERY SearchEngramsFull(
    query: String,
    limit: I64,
    min_importance: F32,
    exclude_contradictions: Boolean
) =>
    candidates <- SearchV<EngramVector>(Embed(query), limit)
        ::In<HasVector>

    filtered <- match exclude_contradictions {
        true => candidates::WHERE(AND(
            _::{importance_score}::GTE(min_importance),
            _::{contradiction_flag}::EQ(false)
        )),
        false => candidates::WHERE(
            _::{importance_score}::GTE(min_importance)
        )
    }

    RETURN filtered::{ID, episode_raw, emotional_charge, importance_score, engram_type, timestamp}

// RUST:
// let results = client.query("SearchEngramsFull", &json!({
//     "query": "infrastructure development in Africa",
//     "limit": 20,
//     "min_importance": 0.3,
//     "exclude_contradictions": true
// })).await?;


// ============================================================
// 4. EBBINGHAUS DECAY & MEMORY CONSOLIDATION
// ============================================================

// Find engrams due for compression (retention < 0.15)
// Application code computes: retention = exp(-(decay_rate * days_elapsed) / sqrt(activation_count + 1))
QUERY GetEngramsForDecayCheck(engram_type: String) =>
    engrams <- N<Engram>::WHERE(AND(
        _::{engram_type}::EQ(engram_type),
        _::{compression_level}::EQ(0)
    ))
    RETURN engrams::{ID, decay_rate, activation_count, timestamp, importance_score}

// RUST (compute retention in app code):
// let engrams = client.query("GetEngramsForDecayCheck", &json!({
//     "engram_type": "episodic"
// })).await?;
// for e in engrams {
//     let days = (now - e.timestamp).days() as f64;
//     let retention = (-e.decay_rate * days / (e.activation_count + 1.0).sqrt()).exp();
//     if retention < 0.15 && e.importance_score < 0.3 {
//         // compress
//     } else if retention < 0.05 && e.importance_score < 0.1 {
//         // prune
//     }
// }

// ──────────────────────────────────────

// Compress an engram (clear raw episode, bump compression_level to 1)
QUERY CompressEngram(engram_id: ID) =>
    compressed <- N<Engram>(engram_id)::UpsertN({
        compression_level: 1,
        episode_raw: "COMPRESSED"
    })
    RETURN compressed

// ──────────────────────────────────────

// Promote engram to semantic memory (crystallized)
QUERY PromoteToSemantic(engram_id: ID) =>
    promoted <- N<Engram>(engram_id)::UpsertN({
        engram_type: "semantic",
        compression_level: 2
    })
    RETURN promoted

// ──────────────────────────────────────

// Prune an engram (drop node, cascade-deletes connected edges)
QUERY PruneEngram(engram_id: ID) =>
    DROP N<Engram>(engram_id)
    RETURN "pruned"

// ──────────────────────────────────────

// Increment activation count on recall
QUERY ActivateEngram(engram_id: ID) =>
    engram <- N<Engram>(engram_id)
    updated <- engram::UpsertN({
        activation_count: ADD(engram::{activation_count}, 1)
    })
    RETURN updated

// ──────────────────────────────────────

// Find concepts ready for mitosis (high source_count via inbound edges)
QUERY CheckMitosisCandidates(min_engram_count: U32) =>
    concepts <- N<Concept>::WHERE(
        _::{source_count}::GTE(min_engram_count)
    )
    RETURN concepts::{ID, name, confidence, source_count}

// ──────────────────────────────────────

// Refresh source_count for a concept (count its incoming Recalls edges)
QUERY UpdateConceptSourceCount(concept_id: ID) =>
    concept <- N<Concept>(concept_id)
    count <- concept::In<Recalls>::COUNT
    updated <- concept::UpsertN({source_count: count})
    RETURN updated


// ============================================================
// 5. IDENTITY CRYSTALLIZATION
// ============================================================

// Get all identity traits
QUERY GetAllIdentity() =>
    traits <- N<Identity>
    RETURN traits::{trait, value, stability, last_updated}

// ──────────────────────────────────────

// Update identity trait value (resists change based on stability)
QUERY UpdateIdentityTrait(
    identity_id: ID,
    new_value: F32,
    stability: F32,
    current_value: F32
) =>
    updated <- N<Identity>(identity_id)::UpsertN({
        value: ADD(
            MUL(current_value, stability),
            MUL(new_value, SUB(1.0, stability))
        ),
        last_updated: NOW
    })
    RETURN updated

// RUST (weighted update respecting stability):
// let trait_data = client.query("GetAllIdentity", &json!({})).await?;
// for t in trait_data {
//     let blended = t.value * t.stability + new_evidence * (1.0 - t.stability);
//     client.query("UpdateIdentityTrait", &json!({
//         "identity_id": t.id,
//         "new_value": new_evidence,
//         "stability": t.stability,
//         "current_value": t.value
//     })).await?;
// }


// ============================================================
// 6. CONTRADICTION DETECTION
// ============================================================

// Find engrams that may contradict each other
// (same concept recalled, but opposite emotional_charge signs)
QUERY DetectContradictionCandidates(concept_id: ID) =>
    engrams <- N<Concept>(concept_id)::In<Recalls>
    RETURN engrams::{ID, episode_raw, emotional_charge, importance_score}

// RUST (post-processing):
// let mut positive = vec![];
// let mut negative = vec![];
// for e in engrams_for_concept {
//     if e.emotional_charge > 0.3 { positive.push(e); }
//     else if e.emotional_charge < -0.3 { negative.push(e); }
// }
// for pos in &positive {
//     for neg in &negative {
//         client.query("ConnectContradiction", &json!({
//             "engram_a_id": pos.id, "engram_b_id": neg.id
//         })).await?;
//     }
// }

// ──────────────────────────────────────

// Get engrams with active contradictions
QUERY GetContradictedEngrams() =>
    engrams <- N<Engram>::WHERE(
        AND(
            _::{contradiction_flag}::EQ(true),
            EXISTS(_::Out<Contradicts>)
        )
    )
    RETURN engrams::{ID, episode_raw, engram_type}


// ============================================================
// 7. TEMPORAL SEQUENCE QUERIES
// ============================================================

// Get the timeline of engrams (ordered by timestamp)
QUERY GetTimeline(limit: I64, offset: I64) =>
    engrams <- N<Engram>::
        ORDER<Desc>(_::{timestamp})::
        RANGE(offset, ADD(offset, limit))
    RETURN engrams::{ID, episode_raw, engram_type, emotional_charge, timestamp}

// ──────────────────────────────────────

// Get temporal chain from a specific engram
QUERY GetTemporalChain(engram_id: ID) =>
    next <- N<Engram>(engram_id)::Out<TemporalSequence>
    prev <- N<Engram>(engram_id)::In<TemporalSequence>
    RETURN {previous: prev, next: next}


// ============================================================
// 8. AGGREGATION & STATISTICS
// ============================================================

// Count engrams by type
QUERY CountEngramsByType() =>
    counts <- N<Engram>::GROUP_BY(engram_type)
    RETURN counts

// ──────────────────────────────────────

// Get memory health stats
QUERY GetMemoryStats() =>
    total <- N<Engram>::COUNT
    contradictions <- N<Engram>::WHERE(
        _::{contradiction_flag}::EQ(true)
    )::COUNT
    concepts <- N<Concept>::COUNT
    identities <- N<Identity>::COUNT
    RETURN {
        total_engrams: total,
        contradiction_count: contradictions,
        concept_count: concepts,
        identity_count: identities
    }
