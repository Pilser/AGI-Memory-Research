// ============================================================
// MNEMOS — Cognitive Memory Operating System
// Full Pipeline Reference (helixdb-embedded-pipeline.rs)
// ============================================================
//
// This is a REFERENCE implementation showing how the entire MNEMOS pipeline
// works with the new helix-db 3.0.0 embedded SDK. It combines:
//   - Embedded client initialization (disk/in-memory/object storage)
//   - Schema creation via Rust DSL (vectors stored DIRECTLY on nodes)
//   - All queries as #[query] functions
//   - Cognitive Resonance Retrieval (CRR)
//   - Consolidation loop (decay, compression, promotion)
//   - Contradiction detection
//   - Identity crystallization
//
// KEY SIMPLIFICATION: Embeddings are stored directly on Engram nodes.
// No separate EngramVector node or HasVector edge needed.
//
// NOT production code — research reference only.
//
// See also:
// - helixdb-embedded-integration.md — SDK setup, client init, storage backends
// - helixdb-embedded-schema.md — nodes, edges, indexes
// - helixdb-embedded-queries.md — all #[query] functions
// - helixdb-embedded-retrieval.md — CRR + Stimulation Layer
// - FutureMemoryAGI.md — system overview and architecture
// ============================================================

use helix_db::{Client, HelixDbSource, CacheConfig, CacheMode};
use helix_db::dsl::prelude::*;
use serde::{Deserialize, Serialize};
use std::num::NonZeroUsize;

// ============================================================
// DATA SHAPES — what comes back from HelixDB queries
// ============================================================

#[derive(Debug, Deserialize)]
struct EngramCandidate {
    #[serde(rename = "$id")]
    id: u64,
    episode_raw: String,
    emotional_charge: f64,
    importance_score: f64,
    decay_rate: f64,
    activation_count: i64,
    timestamp: String,
    engram_type: String,
    compression_level: i64,
    contradiction_flag: bool,
    #[serde(rename = "$distance")]
    distance: f64,
}

#[derive(Debug, Deserialize)]
struct ConceptData {
    #[serde(rename = "$id")]
    id: u64,
    name: String,
    confidence: f64,
    source_count: i64,
}

#[derive(Debug, Deserialize)]
struct IdentityTrait {
    #[serde(rename = "$id")]
    id: u64,
    #[serde(rename = "trait")]
    trait_name: String,
    value: f64,
    stability: f64,
}

/// Engram candidate with embedding (for centroid computation)
#[derive(Debug, Deserialize)]
struct EngramCandidateWithEmbedding {
    #[serde(rename = "$id")]
    id: u64,
    episode_raw: String,
    emotional_charge: f64,
    importance_score: f64,
    embedding: Vec<f32>,  // ← includes embedding for centroid computation
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

// ============================================================
// ML MODEL INTERFACES (custom, NOT in HelixDB)
// These are integration points — implement with your choice of
// ONNX / candle / llama.cpp / remote API
// ============================================================

struct ImportanceScorer;

impl ImportanceScorer {
    fn new() -> Self { Self }

    /// Score a batch of engrams. Returns 0.0–1.0 per engram.
    fn score_batch(&self, episodes: &[String]) -> Vec<f64> {
        // Option A: ONNX Runtime (ort crate)
        // Option B: candle (Rust-native ML)
        // Option C: llama.cpp / mistral.rs (local LLM)
        // Option D: Remote API (OpenAI/Anthropic)
        vec![0.5; episodes.len()]  // PLACEHOLDER
    }
}

struct EmotionalTagger;

impl EmotionalTagger {
    fn new() -> Self { Self }

    /// Returns -1.0 (negative) to +1.0 (positive) per text
    fn tag_batch(&self, episodes: &[String]) -> Vec<f64> {
        vec![0.0; episodes.len()]  // PLACEHOLDER
    }
}

struct ConceptExtractor;

#[derive(Debug)]
struct ExtractedConcept {
    name: String,
    confidence: f64,
}

impl ConceptExtractor {
    fn new() -> Self { Self }

    fn extract(&self, text: &str) -> Vec<ExtractedConcept> {
        // LLM prompt: "Extract key concepts from this memory.
        // Return JSON list of {name, confidence}."
        vec![]  // PLACEHOLDER
    }
}

struct EmbeddingModel;

impl EmbeddingModel {
    fn new() -> Self { Self }

    /// Generate embedding vector for text. Returns 1536-dim vector.
    fn embed(&self, text: &str) -> Vec<f32> {
        // Option A: OpenAI text-embedding-3-small (1536-dim)
        // Option B: Local model (e.g., all-MiniLM-L6-v2, then project to 1536)
        // Option C: Custom fine-tuned embedding model
        vec![0.0; 1536]  // PLACEHOLDER
    }
}

// ============================================================
// QUERY DEFINITIONS — all queries as #[query] functions
// ============================================================

/// Create an engram node with all properties
#[query]
fn create_engram(
    episode_raw: String,
    emotional_charge: f64,
    importance_score: f64,
    decay_rate: f64,
    activation_count: i64,
    engram_type: String,
    compression_level: i64,
    contradiction_flag: bool,
    timestamp: String,
) -> WriteBatch {
    write_batch()
        .var_as("engram", g().add_n("Engram", vec![
            ("episode_raw", episode_raw),
            ("emotional_charge", emotional_charge),
            ("importance_score", importance_score),
            ("decay_rate", decay_rate),
            ("activation_count", activation_count),
            ("timestamp", timestamp),
            ("engram_type", engram_type),
            ("compression_level", compression_level),
            ("contradiction_flag", contradiction_flag),
        ]))
        .returning(["engram"])
}

/// Create a concept node
#[query]
fn create_concept(
    name: String,
    confidence: f64,
    formation_date: String,
) -> WriteBatch {
    write_batch()
        .var_as("concept", g().add_n("Concept", vec![
            ("name", name),
            ("confidence", confidence),
            ("formation_date", formation_date),
            ("source_count", 1_i64),
        ]))
        .returning(["concept"])
}

/// Create an identity trait node
#[query]
fn create_identity(
    trait_name: String,
    value: f64,
    stability: f64,
    last_updated: String,
) -> WriteBatch {
    write_batch()
        .var_as("identity", g().add_n("Identity", vec![
            ("trait", trait_name),
            ("value", value),
            ("stability", stability),
            ("last_updated", last_updated),
        ]))
        .returning(["identity"])
}

/// Create edge between two nodes by ID
#[query]
fn connect_nodes(
    from_id: u64,
    to_id: u64,
    edge_label: String,
) -> WriteBatch {
    write_batch()
        .var_as("edge", g()
            .n(NodeRef::id(from_id))
            .add_e(edge_label, NodeRef::id(to_id), vec![]))
        .returning(["edge"])
}

/// Optional: Create Recalls edge with relationship embedding (for dynamic Stimulation Layer weights)
#[query]
fn connect_recalls_with_embedding(
    engram_id: u64,
    concept_id: u64,
    relationship_embedding: Vec<f32>,
    strength: f64,
) -> WriteBatch {
    write_batch()
        .var_as("edge", g()
            .n(NodeRef::id(engram_id))
            .add_e("Recalls", NodeRef::id(concept_id), vec![
                ("relationship_embedding", relationship_embedding),
                ("strength", strength),
            ]))
        .returning(["edge"])
}

/// Optional: Update concept centroid embedding (call after mitosis or engram reassignment)
#[query]
fn update_concept_centroid(
    concept_id: u64,
    centroid_embedding: Vec<f32>,
) -> WriteBatch {
    write_batch()
        .var_as("updated", g()
            .n(NodeRef::id(concept_id))
            .set_property("embedding", centroid_embedding))
        .returning(["updated"])
}

/// Atomic: create engram with embedding stored directly on node
#[query]
fn create_full_engram(
    episode_raw: String,
    emotional_charge: f64,
    importance_score: f64,
    decay_rate: f64,
    engram_type: String,
    compression_level: i64,
    contradiction_flag: bool,
    timestamp: String,
    embedding: Vec<f32>,
) -> WriteBatch {
    write_batch()
        // Engram node with embedding stored DIRECTLY on it (no separate vector node)
        .var_as("engram", g().add_n("Engram", vec![
            ("episode_raw", episode_raw),
            ("emotional_charge", emotional_charge),
            ("importance_score", importance_score),
            ("decay_rate", decay_rate),
            ("activation_count", 0_i64),
            ("timestamp", timestamp),
            ("engram_type", engram_type),
            ("compression_level", compression_level),
            ("contradiction_flag", contradiction_flag),
            ("embedding", embedding),  // ← vector stored directly on Engram node
        ]))
        .returning(["engram"])
}

/// Get engram by ID
#[query]
fn get_engram(engram_id: u64) -> ReadBatch {
    read_batch()
        .var_as("engram", g()
            .n(NodeRef::id(engram_id))
            .value_map(Some(vec![
                "$id", "episode_raw", "emotional_charge", "importance_score",
                "decay_rate", "activation_count", "timestamp", "engram_type",
                "compression_level", "contradiction_flag",
            ])))
        .returning(["engram"])
}

/// Get all engrams of a given type
#[query]
fn get_engrams_by_type(engram_type: String) -> ReadBatch {
    read_batch()
        .var_as("engrams", g()
            .n_with_label_where("Engram", SourcePredicate::eq("engram_type", engram_type))
            .value_map(Some(vec![
                "$id", "episode_raw", "emotional_charge",
                "importance_score", "timestamp",
            ])))
        .returning(["engrams"])
}

/// Get concept by name
#[query]
fn get_concept_by_name(name: String) -> ReadBatch {
    read_batch()
        .var_as("concept", g()
            .n_with_label_where("Concept", SourcePredicate::eq("name", name))
            .limit(1)
            .value_map(Some(vec!["$id", "name", "confidence", "source_count"])))
        .returning(["concept"])
}

/// Get concepts an engram recalls
#[query]
fn get_recalled_concepts(engram_id: u64) -> ReadBatch {
    read_batch()
        .var_as("concepts", g()
            .n(NodeRef::id(engram_id))
            .out(Some("Recalls"))
            .value_map(Some(vec!["$id", "name", "confidence"])))
        .returning(["concepts"])
}

/// Get the cognitive chain: engram → concepts → identity
#[query]
fn get_cognitive_chain(engram_id: u64) -> ReadBatch {
    read_batch()
        .var_as("engram", g()
            .n(NodeRef::id(engram_id))
            .value_map(Some(vec![
                "$id", "episode_raw", "emotional_charge",
                "importance_score", "engram_type",
            ])))
        .var_as("concepts", g()
            .n(NodeRef::id(engram_id))
            .out(Some("Recalls"))
            .value_map(Some(vec!["$id", "name", "confidence"])))
        .var_as("identities", g()
            .n(NodeRef::id(engram_id))
            .out(Some("Recalls"))
            .out(Some("Defines"))
            .value_map(Some(vec!["$id", "trait", "value", "stability"])))
        .returning(["engram", "concepts", "identities"])
}

/// Vector search + filter (no traversal needed — embedding on Engram directly)
#[query]
fn search_engrams_full(
    query_embedding: Vec<f32>,
    limit: i64,
    min_importance: f64,
) -> ReadBatch {
    read_batch()
        .var_as("candidates", g()
            // Vector search returns Engram nodes DIRECTLY
            .vector_search_with("Engram", "embedding", query_embedding, limit, None)
            // Filter directly on Engram properties (no ::In<HasVector> needed)
            .where_(Predicate::gte("importance_score", min_importance))
            .where_(Predicate::eq("contradiction_flag", false))
            .value_map(Some(vec![
                "$id", "episode_raw", "emotional_charge",
                "importance_score", "engram_type", "timestamp",
            ])))
        .returning(["candidates"])
}

/// Get engrams for decay check
#[query]
fn get_engrams_for_decay_check(engram_type: String) -> ReadBatch {
    read_batch()
        .var_as("engrams", g()
            .n_with_label_where("Engram", SourcePredicate::eq("engram_type", engram_type))
            .where_(Predicate::eq("compression_level", 0_i64))
            .value_map(Some(vec![
                "$id", "decay_rate", "activation_count",
                "timestamp", "importance_score",
            ])))
        .returning(["engrams"])
}

/// Compress an engram
#[query]
fn compress_engram(engram_id: u64) -> WriteBatch {
    write_batch()
        .var_as("compressed", g()
            .n(NodeRef::id(engram_id))
            .set_property("compression_level", 1_i64)
            .set_property("episode_raw", "COMPRESSED"))
        .returning(["compressed"])
}

/// Promote engram to semantic memory
#[query]
fn promote_to_semantic(engram_id: u64) -> WriteBatch {
    write_batch()
        .var_as("promoted", g()
            .n(NodeRef::id(engram_id))
            .set_property("engram_type", "semantic")
            .set_property("compression_level", 2_i64))
        .returning(["promoted"])
}

/// Prune (delete) an engram
#[query]
fn prune_engram(engram_id: u64) -> WriteBatch {
    write_batch()
        .var_as("pruned", g()
            .n(NodeRef::id(engram_id))
            .drop())
        .returning(["pruned"])
}

/// Activate an engram (increment activation_count)
#[query]
fn activate_engram(
    engram_id: u64,
    current_count: i64,
) -> WriteBatch {
    write_batch()
        .var_as("updated", g()
            .n(NodeRef::id(engram_id))
            .set_property("activation_count", current_count + 1))
        .returning(["updated"])
}

/// Get all identity traits
#[query]
fn get_all_identity() -> ReadBatch {
    read_batch()
        .var_as("traits", g()
            .n_with_label("Identity")
            .value_map(Some(vec!["$id", "trait", "value", "stability", "last_updated"])))
        .returning(["traits"])
}

/// Update identity trait with weighted blend
#[query]
fn update_identity_trait(
    identity_id: u64,
    blended_value: f64,
    last_updated: String,
) -> WriteBatch {
    write_batch()
        .var_as("updated", g()
            .n(NodeRef::id(identity_id))
            .set_property("value", blended_value)
            .set_property("last_updated", last_updated))
        .returning(["updated"])
}

/// Get engrams recalling a concept (for contradiction detection)
#[query]
fn get_engrams_recalling_concept(concept_id: u64) -> ReadBatch {
    read_batch()
        .var_as("engrams", g()
            .n(NodeRef::id(concept_id))
            .in_(Some("Recalls"))
            .value_map(Some(vec![
                "$id", "episode_raw", "emotional_charge", "importance_score",
            ])))
        .returning(["engrams"])
}

/// Optional: Get engrams with embeddings (for centroid computation)
#[query]
fn get_engrams_recalling_concept_with_embeddings(concept_id: u64) -> ReadBatch {
    read_batch()
        .var_as("engrams", g()
            .n(NodeRef::id(concept_id))
            .in_(Some("Recalls"))
            .value_map(Some(vec![
                "$id", "episode_raw", "emotional_charge", "importance_score", "embedding",
            ])))
        .returning(["engrams"])
}

/// Get memory statistics
#[query]
fn get_memory_stats() -> ReadBatch {
    read_batch()
        .var_as("total", g().n_with_label("Engram").count())
        .var_as("contradictions", g()
            .n_with_label_where("Engram", SourcePredicate::eq("contradiction_flag", true))
            .count())
        .var_as("concepts", g().n_with_label("Concept").count())
        .var_as("identities", g().n_with_label("Identity").count())
        .returning(["total", "contradictions", "concepts", "identities"])
}

/// Create vector index on Engram.embedding (must be called once at setup)
#[query]
fn create_vector_index() -> WriteBatch {
    write_batch()
        .var_as("index", g().create_vector_index_nodes(
            "Engram",    // label — vector stored directly on Engram node
            "embedding", // property
            NonZeroUsize::new(1536).expect("non-zero"),
            VectorDistanceMetric::Cosine,
            None::<&str>,
        ))
        .returning(["index"])
}

/// Create secondary index on Engram.engram_type
#[query]
fn create_engram_type_index() -> WriteBatch {
    write_batch()
        .var_as("index", g().create_index_if_not_exists(
            IndexSpec::node_equality("Engram", "engram_type")
        ))
        .returning(["index"])
}

/// Create secondary index on Engram.contradiction_flag
#[query]
fn create_contradiction_index() -> WriteBatch {
    write_batch()
        .var_as("index", g().create_index_if_not_exists(
            IndexSpec::node_equality("Engram", "contradiction_flag")
        ))
        .returning(["index"])
}

/// Create secondary index on Concept.name
#[query]
fn create_concept_name_index() -> WriteBatch {
    write_batch()
        .var_as("index", g().create_index_if_not_exists(
            IndexSpec::node_equality("Concept", "name")
        ))
        .returning(["index"])
}

/// Optional: Create vector index on Concept.embedding (for concept-level search)
#[query]
fn create_concept_vector_index() -> WriteBatch {
    write_batch()
        .var_as("index", g().create_vector_index_nodes(
            "Concept",                                     // label
            "embedding",                                   // property (centroid embedding)
            NonZeroUsize::new(1536).expect("non-zero"),    // dimension
            VectorDistanceMetric::Cosine,                  // distance metric
            None::<&str>,                                  // no tenant partitioning
        ))
        .returning(["index"])
}

// ============================================================
// PIPELINE ORCHESTRATOR — the main loop
// ============================================================

struct MemoryPipeline {
    client: Client,
    importance_scorer: ImportanceScorer,
    emotional_tagger: EmotionalTagger,
    concept_extractor: ConceptExtractor,
    embedding_model: EmbeddingModel,
}

impl MemoryPipeline {
    async fn new(client: Client) -> Self {
        Self {
            client,
            importance_scorer: ImportanceScorer::new(),
            emotional_tagger: EmotionalTagger::new(),
            concept_extractor: ConceptExtractor::new(),
            embedding_model: EmbeddingModel::new(),
        }
    }

    // --- INGESTION PIPE: raw text → scored engram in HelixDB ---
    async fn ingest_memory(
        &self,
        episode_raw: &str,
        engram_type: &str,
    ) -> Result<u64, Box<dyn std::error::Error>> {
        // Step 1: Run ML models on the raw text
        let importance = self.importance_scorer.score_batch(&[episode_raw.to_string()])[0];
        let emotional = self.emotional_tagger.tag_batch(&[episode_raw.to_string()])[0];
        let embedding = self.embedding_model.embed(episode_raw);

        // Step 2: Store in EngramDB (atomic: engram with embedding directly on node)
        // No separate vector node or HasVector edge needed!
        let timestamp = chrono::Utc::now().to_rfc3339();
        let request = create_full_engram(
            episode_raw.to_string(),
            emotional,
            importance,
            0.05,  // default decay rate
            engram_type.to_string(),
            0,     // compression_level
            false, // contradiction_flag
            timestamp,
            embedding,
        );

        let response = self.client.query(request).send().await?;
        let engram_id = response["engram"]["$id"].as_u64().unwrap();

        // Step 3: Extract concepts and link them
        let concepts = self.concept_extractor.extract(episode_raw);
        for concept in &concepts {
            // Create concept (or get existing)
            let concept_request = create_concept(
                concept.name.clone(),
                concept.confidence,
                chrono::Utc::now().to_rfc3339(),
            );
            let concept_response = self.client.query(concept_request).send().await?;
            let concept_id = concept_response["concept"]["$id"].as_u64().unwrap();

            // Link engram → concept (Recalls)
            let recalls_request = connect_nodes(engram_id, concept_id, "Recalls".to_string());
            self.client.query(recalls_request).send().await?;

            // Link engram → concept (AbstractsTo)
            let abstracts_request = connect_nodes(engram_id, concept_id, "AbstractsTo".to_string());
            self.client.query(abstracts_request).send().await?;
        }

        Ok(engram_id)
    }

    // --- RETRIEVAL PIPE: query → resonance-scored results ---
    async fn retrieve_memories(
        &self,
        query_text: &str,
        limit: usize,
    ) -> Result<Vec<ResonanceResult>, Box<dyn std::error::Error>> {
        // Step 1: Generate embedding for the query
        let query_embedding = self.embedding_model.embed(query_text);

        // Step 2: Vector search (returns Engram directly — no traversal needed)
        let search_request = search_engrams_full(query_embedding.clone(), 20, 0.0);
        let search_response = self.client.query(search_request).send().await?;

        // Step 3: Deserialize candidates
        let candidates: Vec<EngramCandidate> = serde_json::from_value(
            search_response["candidates"].clone()
        )?;

        // Step 4: Fetch identity graph (for alignment scoring)
        let identity_request = get_all_identity();
        let identity_response = self.client.query(identity_request).send().await?;
        let identities: Vec<IdentityTrait> = serde_json::from_value(
            identity_response["traits"].clone()
        )?;

        // Step 5: Compute semantic similarities (1.0 - cosine distance)
        let semantic_sims: Vec<f64> = candidates.iter()
            .map(|c| 1.0 - c.distance)
            .collect();

        // Step 6: Compute resonance scores
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs_f64();

        let mut results = compute_resonance(&candidates, &identities, now, &semantic_sims);

        // Step 7: Sort by resonance (highest first)
        results.sort_by(|a, b| b.resonance_score.partial_cmp(&a.resonance_score).unwrap());

        // Step 8: Take top N
        results.truncate(limit);

        // Step 9: Activate recalled engrams (bump activation_count)
        for r in &results {
            if r.resonance_score > 0.3 {
                // Get current activation count
                let get_request = get_engram(r.engram_id);
                if let Ok(get_response) = self.client.query(get_request).send().await {
                    if let Some(count) = get_response["engram"]["activation_count"].as_i64() {
                        let activate_request = activate_engram(r.engram_id, count);
                        let _ = self.client.query(activate_request).send().await;
                    }
                }
            }
        }

        Ok(results)
    }

    // --- CONSOLIDATION LOOP (runs between sessions, like "sleep") ---
    async fn consolidate(&self) -> Result<(), Box<dyn std::error::Error>> {
        println!("=== Starting consolidation cycle ===");

        // 1. DECAY: check all episodic engrams for Ebbinghaus decay
        let decay_request = get_engrams_for_decay_check("episodic".to_string());
        let decay_response = self.client.query(decay_request).send().await?;
        let engrams: Vec<EngramCandidate> = serde_json::from_value(
            decay_response["engrams"].clone()
        )?;

        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs_f64();

        for e in &engrams {
            let engram_ts = parse_timestamp(&e.timestamp);
            let days_elapsed = (now - engram_ts) / 86400.0;
            let retention = (-e.decay_rate * days_elapsed
                / (e.activation_count as f64 + 1.0).sqrt())
                .exp();

            if retention < 0.05 && e.importance_score < 0.1 {
                // PRUNE: delete the engram
                println!("  Pruning: {} (retention={:.4})", e.id, retention);
                let prune_request = prune_engram(e.id);
                self.client.query(prune_request).send().await?;
            } else if retention < 0.15 && e.importance_score < 0.3 {
                // COMPRESS: keep structure, lose raw text
                println!("  Compressing: {} (retention={:.4})", e.id, retention);
                let compress_request = compress_engram(e.id);
                self.client.query(compress_request).send().await?;
            } else if retention > 0.8 && e.activation_count > 20 {
                // PROMOTE: to semantic memory
                println!("  Promoting: {} (retention={:.4})", e.id, retention);
                let promote_request = promote_to_semantic(e.id);
                self.client.query(promote_request).send().await?;
            }
        }

        // 2. CONTRADICTION DETECTION: scan for opposing engrams
        self.detect_contradictions().await?;

        println!("=== Consolidation cycle complete ===");
        Ok(())
    }

    async fn detect_contradictions(
        &self,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get all concepts
        let concepts_request = read_batch()
            .var_as("concepts", g()
                .n_with_label("Concept")
                .value_map(Some(vec!["$id", "name"])))
            .returning(["concepts"]);

        let concepts_response = self.client.query(concepts_request).send().await?;
        let concepts: Vec<ConceptData> = serde_json::from_value(
            concepts_response["concepts"].clone()
        )?;

        for concept in &concepts {
            // Get all engrams recalling this concept
            let engrams_request = get_engrams_recalling_concept(concept.id);
            let engrams_response = self.client.query(engrams_request).send().await?;
            let engrams: Vec<EngramCandidate> = serde_json::from_value(
                engrams_response["engrams"].clone()
            )?;

            // Group by emotional charge sign
            let positive: Vec<&EngramCandidate> = engrams.iter()
                .filter(|e| e.emotional_charge > 0.3)
                .collect();
            let negative: Vec<&EngramCandidate> = engrams.iter()
                .filter(|e| e.emotional_charge < -0.3)
                .collect();

            // Create contradiction edges between opposing pairs
            for pos in &positive {
                for neg in &negative {
                    let contradiction_request = connect_nodes(
                        pos.id,
                        neg.id,
                        "Contradicts".to_string(),
                    );
                    let _ = self.client.query(contradiction_request).send().await;
                }
            }
        }

        Ok(())
    }

    // --- IDENTITY CRYSTALLIZATION ---
    async fn update_identity_from_experience(
        &self,
        engram_id: u64,
    ) -> Result<(), Box<dyn std::error::Error>> {
        // Get engram → concepts → identity chain
        let chain_request = get_cognitive_chain(engram_id);
        let chain_response = self.client.query(chain_request).send().await?;

        // Parse identity traits from chain
        let identities: Vec<IdentityTrait> = serde_json::from_value(
            chain_response["identities"].clone()
        )?;

        // Apply weighted update respecting stability
        for trait_data in &identities {
            // In a real implementation, you'd compute new_evidence from the engram
            let new_evidence = 0.5; // placeholder
            let blended = trait_data.value * trait_data.stability
                + new_evidence * (1.0 - trait_data.stability);

            let update_request = update_identity_trait(
                trait_data.id,
                blended,
                chrono::Utc::now().to_rfc3339(),
            );
            let _ = self.client.query(update_request).send().await;
        }

        Ok(())
    }
}

// ============================================================
// RESONANCE SCORING — pure math, NOT in HelixDB
// ============================================================

fn compute_resonance(
    candidates: &[EngramCandidate],
    identities: &[IdentityTrait],
    now_ts: f64,
    semantic_sims: &[f64],
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
        let contradiction_factor = if c.contradiction_flag { 0.5 } else { 1.0 };

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
    _text: &str,
    identities: &[IdentityTrait],
) -> f64 {
    if identities.is_empty() {
        return 1.0;
    }
    // Compare text embedding against identity trait embeddings.
    // Simple version: return 1.0 (neutral) until identity graph is built.
    // Advanced: embed text, cosine-sim against each identity trait's embedding.
    1.0
}

// ============================================================
// OPTIONAL ENHANCEMENT: Concept Centroid Computation
// ============================================================

/// Compute centroid embedding from a list of engram embeddings
/// Call this after mitosis creates child concepts or engrams are reassigned
fn compute_centroid(embeddings: &[Vec<f32>]) -> Vec<f32> {
    if embeddings.is_empty() {
        return vec![0.0; 1536];
    }
    let dim = embeddings[0].len();
    let mut centroid = vec![0.0f32; dim];
    for embedding in embeddings {
        for (i, val) in embedding.iter().enumerate() {
            if i < dim {
                centroid[i] += val;
            }
        }
    }
    let n = embeddings.len() as f32;
    for val in &mut centroid {
        *val /= n;
    }
    centroid
}

/// Compute and update centroid for a concept (call after mitosis or engram reassignment)
async fn compute_and_update_centroid(
    client: &Client,
    concept_id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Get all engrams recalling this concept (with embeddings)
    let engrams_request = get_engrams_recalling_concept_with_embeddings(concept_id);
    let engrams_response = client.query(engrams_request).send().await?;
    let engrams: Vec<EngramCandidateWithEmbedding> = serde_json::from_value(
        engrams_response["engrams"].clone()
    )?;

    // 2. Extract embeddings
    let embeddings: Vec<Vec<f32>> = engrams.iter()
        .map(|e| e.embedding.clone())
        .collect();

    // 3. Compute centroid
    let centroid = compute_centroid(&embeddings);

    // 4. Store centroid on the concept
    let update_request = update_concept_centroid(concept_id, centroid);
    client.query(update_request).send().await?;

    Ok(())
}

// ============================================================
// OPTIONAL ENHANCEMENT: Dynamic Edge Weights for Stimulation Layer
// ============================================================

/// Compute relationship strength between an edge embedding and concept centroid
/// Returns value between 0.0 (weak) and 1.0 (strong)
fn compute_relationship_strength(
    edge_embedding: &[f32],
    concept_centroid: &[f32],
) -> f64 {
    cosine_similarity(edge_embedding, concept_centroid)
}

/// Cosine similarity between two vectors
fn cosine_similarity(a: &[f32], b: &[f32]) -> f64 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    let mut dot = 0.0f64;
    let mut norm_a = 0.0f64;
    let mut norm_b = 0.0f64;
    for i in 0..a.len() {
        let ai = a[i] as f64;
        let bi = b[i] as f64;
        dot += ai * bi;
        norm_a += ai * ai;
        norm_b += bi * bi;
    }
    let denom = (norm_a * norm_b).sqrt();
    if denom < 1e-10 {
        return 0.0;
    }
    dot / denom
}

/// Compute dynamic activation transfer for an edge (instead of fixed α weight)
/// This makes the Stimulation Layer more biologically plausible
fn compute_activation_transfer(
    source_activation: f64,
    edge_embedding: &[f32],
    concept_centroid: &[f32],
    alpha_base: f64,
) -> f64 {
    let relationship_strength = compute_relationship_strength(edge_embedding, concept_centroid);
    source_activation * relationship_strength * alpha_base
}

fn parse_timestamp(ts: &str) -> f64 {
    chrono::DateTime::parse_from_rfc3339(ts)
        .map(|dt| dt.timestamp() as f64)
        .unwrap_or(0.0)
}

// ============================================================
// DATABASE SETUP — initialize embedded database with indexes
// ============================================================

async fn setup_database(client: &Client) -> Result<(), Box<dyn std::error::Error>> {
    // Create vector index on Engram.embedding (required for search)
    let vector_index_request = create_vector_index();
    let _ = client.query(vector_index_request).send().await?;
    // NOTE: In production, poll until index is active before proceeding

    // Create secondary indexes for efficient filtering
    let engram_type_index_request = create_engram_type_index();
    let _ = client.query(engram_type_index_request).send().await?;

    let contradiction_index_request = create_contradiction_index();
    let _ = client.query(contradiction_index_request).send().await?;

    let concept_name_index_request = create_concept_name_index();
    let _ = client.query(concept_name_index_request).send().await?;

    // Optional: Create vector index on Concept.embedding (for concept-level search)
    // Only needed if using concept centroid enhancement
    // let concept_vector_index_request = create_concept_vector_index();
    // let _ = client.query(concept_vector_index_request).send().await?;

    println!("Database setup complete (indexes created)");
    Ok(())
}

// ============================================================
// MAIN — example usage
// ============================================================

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ── STORAGE BACKEND SELECTION ──

    // Option A: Disk (default for production)
    let client = Client::open(HelixDbSource::Disk {
        root: "/data/helix".into(),
        database: "mnemos".to_string(),
    }).await?;

    // Option B: In-memory (for tests)
    // let client = Client::open(HelixDbSource::InMemory {
    //     database: "mnemos".to_string(),
    // }).await?;

    // Option C: Object storage (for durable shared storage)
    // let client = Client::open(HelixDbSource::ObjectStorage {
    //     database: "mnemos".to_string(),
    //     bucket: "helix-production".into(),
    //     region: "eu-west-2".into(),
    // }).await?;

    // Option D: Custom cache configuration
    // let client = Client::open(
    //     HelixDbSource::Disk {
    //         root: "/data/helix".into(),
    //         database: "mnemos".to_string(),
    //     },
    //     Some(CacheConfig {
    //         vector_memory_bytes: 512 * 1024 * 1024,
    //         mode: CacheMode::Memory,
    //     }),
    // ).await?;

    // ── DATABASE SETUP ──
    setup_database(&client).await?;

    // ── PIPELINE INITIALIZATION ──
    let pipeline = MemoryPipeline::new(client).await;

    // ── INGEST A MEMORY ──
    let engram_id = pipeline.ingest_memory(
        "User talked about Uganda's ICT hub and its impact on East African infrastructure",
        "episodic"
    ).await?;
    println!("Ingested engram: {}", engram_id);

    // ── RETRIEVE RELEVANT MEMORIES ──
    let results = pipeline.retrieve_memories(
        "African infrastructure development",
        10
    ).await?;

    for r in &results {
        println!(
            "Resonance={:.4} | id={} | episode={:.60}...",
            r.resonance_score, r.engram_id, r.episode_raw
        );
    }

    // ── RUN CONSOLIDATION (typically a background cron/scheduled task) ──
    pipeline.consolidate().await?;

    Ok(())
}
