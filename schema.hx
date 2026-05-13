// ============================================================
// MNEMOS — Cognitive Memory Operating System
// HelixDB Schema Definition (schema.hx)
// ============================================================

// --- Core Memory Unit ---
N::Engram {
    episode_raw:       String,
    emotional_charge:  F32,           // -1.0 to +1.0
    importance_score:  F32,           // 0.0 to 1.0
    decay_rate:        F32,           // Ebbinghaus parameter
    activation_count:  U32,           // times recalled
    timestamp:         Date,          // creation time
    engram_type:       String,        // "episodic" | "semantic" | "procedural"
    compression_level: U8,            // 0=raw, 1=compressed, 2=crystallized
    contradiction_flag: Boolean
}

// --- Concept Node ---
N::Concept {
    name:              String,
    confidence:        F32,
    formation_date:    Date,
    source_count:      U32
}

// --- Identity Trait Node ---
N::Identity {
    trait:             String,
    value:             F32,
    stability:         F32,
    last_updated:      Date
}

// --- Vector Embeddings (separate entity, connected via edge) ---
// semantic_vector lives here, not as a field on Engram
V::EngramVector {
    content: String
}

// ============================================================
// RELATIONSHIPS (Edges)
// ============================================================

// Engram → Concept: engram recalls/supports a concept
E::Recalls {
    From: Engram,
    To: Concept
}

// Engram → Engram: contradiction between engrams
E::Contradicts {
    From: Engram,
    To: Engram
}

// Engram → Engram: mutual reinforcement
E::Reinforces {
    From: Engram,
    To: Engram
}

// Engram → Concept: engram abstracts to a concept
E::AbstractsTo {
    From: Engram,
    To: Concept
}

// Concept → Identity: concept defines an identity trait
E::Defines {
    From: Concept,
    To: Identity
}

// Engram → Engram: temporal ordering
E::TemporalSequence {
    From: Engram,
    To: Engram
}

// Engram → Engram: after memory mitosis (parent → child)
E::SpawnedFrom {
    From: Engram,
    To: Engram
}

// Engram → EngramVector: connects engram to its embedding
E::HasVector {
    From: Engram,
    To: EngramVector
}
