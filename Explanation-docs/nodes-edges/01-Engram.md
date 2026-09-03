# N::Engram — The Cognitive Memory Unit

## What It Is

The engram is the **fundamental unit of memory** in MNEMOS. It represents a single discrete experience, thought, fact, or observation — analogous to what neuroscience calls an "engram" (the physical trace of a memory in the brain). Every interaction the AI has that's worth remembering becomes an Engram node.

---

## Schema

```rust
g().add_n("Engram", vec![
    ("episode_raw", episode_raw),           // String — raw memory text
    ("emotional_charge", emotional_charge), // F64 — -1.0 to +1.0
    ("importance_score", importance_score), // F64 — 0.0 to 1.0
    ("decay_rate", decay_rate),             // F64 — Ebbinghaus parameter
    ("activation_count", activation_count), // I64 — times recalled (use I64 for U32)
    ("timestamp", timestamp),               // String (RFC 3339 date-time)
    ("engram_type", engram_type),           // String — "episodic"|"semantic"|"procedural"
    ("compression_level", compression_level), // I64 — 0=raw, 1=compressed, 2=crystallized
    ("contradiction_flag", contradiction_flag), // bool
    ("embedding", embedding),               // Vec<f32> — 1536-dimension embedding (DIRECTLY ON NODE)
])
```

**Key change:** The embedding is stored directly on the Engram node. No separate `EngramVector` node or `HasVector` edge needed.

---

## Fields Explained

| Field | Set By | Lifecycle |
|---|---|---|
| `episode_raw` | User/system input | Raw at creation. Set to `"COMPRESSED"` when compression_level ≥ 1. |
| `emotional_charge` | Emotional Tagger (ML) | Set at creation. Read during retrieval for resonance boost. |
| `importance_score` | Importance Scorer (ML) | Set at creation. Determines prune/compress/promote decisions. |
| `decay_rate` | System default (0.05) | Set at creation. Controls how fast the memory fades. Higher = faster forgetting. |
| `activation_count` | Resonance Scorer (+1 per recall) | Starts at 0. Incremented every time the engram is retrieved. Higher count → slower decay. |
| `timestamp` | System (`NOW`) | Set at creation. Used for recency weighting in CRR and decay calculations. |
| `engram_type` | System (based on context) | `"episodic"` (experience), `"semantic"` (knowledge), `"procedural"` (how-to). Can be promoted from episodic → semantic. |
| `compression_level` | Consolidation Loop | `0` = full raw text. `1` = compressed (raw text cleared, summary kept). `2` = crystallized (permanent semantic memory). |
| `contradiction_flag` | Contradiction Detector | `false` at creation. Set to `true` when a contradicting engram is detected. |
| `embedding` | Embedding Model (at creation) | 1536-dimension `Vec<f32>` stored directly on the node. Used for semantic vector search. Remains even when `episode_raw` is cleared during compression. |

---

## Outgoing Edges (What an Engram Can Point To)

```
Engram ──Recalls──────────→ Concept         (what concepts this memory relates to)
Engram ──AbstractsTo──────→ Concept         (what concepts this memory feeds into)
Engram ──Contradicts──────→ Engram          (this memory contradicts another)
Engram ──Reinforces───────→ Engram          (this memory reinforces another)
Engram ──TemporalSequence─→ Engram          (this memory happened before that one)
Engram ──SpawnedFrom──────→ Engram          (this is a child of parent memory after mitosis)
```

> **Note:** The `HasVector` outgoing edge has been eliminated. The embedding is now stored directly on the Engram node as the `embedding` property.

## Incoming Edges (What Points to an Engram)

```
Concept      ──Recalls──────────← Engram    (this concept is recalled by engrams)
Concept      ──AbstractsTo──────← Engram    (this concept is built from engrams)
Engram       ──Contradicts──────← Engram    (another memory contradicts this one)
Engram       ──Reinforces───────← Engram    (another memory reinforces this one)
Engram       ──TemporalSequence─← Engram    (a memory happened before this one)
Engram       ──SpawnedFrom──────← Engram    (another memory spawned this one via mitosis)
```

> **Note:** The `HasVector` incoming edge from `EngramVector` has been eliminated along with the separate vector node type.

---

## What It's Used For

| Use Case | How |
|---|---|
| **Memory storage** | `CreateEngram` / `CreateFullEngram` creates the node |
| **Memory retrieval** | Vector search on `Engram.embedding` finds relevant engrams directly |
| **Memory decay** | Consolidation loop reads `decay_rate`, `timestamp`, `activation_count` to compute retention |
| **Memory pruning** | Drop the Engram node — no separate vector to clean up |
| **Memory compression** | `CompressEngram` sets `compression_level=1`, clears `episode_raw` |
| **Memory promotion** | `PromoteToSemantic` sets `engram_type="semantic"`, `compression_level=2` |
| **Recall tracking** | `ActivateEngram` bumps `activation_count` when retrieved |
| **Identity alignment** | Graph traversal walks `Engram → Concept → Identity` to check alignment |
| **Temporal queries** | `GetTimeline` sorts engrams by `timestamp` for chronological memory |

---

## Lifecycle States

```
┌─────────┐    ingestion      ┌──────────────┐
│  (none)  │ ───────────────→ │ RAW (level 0) │
└─────────┘                   │ full text      │
                              └──────┬───────┘
                                     │
                    ┌────────────────┼────────────────┐
                    │                │                │
               retention        retention         retention
               < 0.15 &         < 0.05 &           > 0.8 &
               importance       importance        activations
               < 0.3             < 0.1             > 20
                    │                │                │
                    ▼                ▼                ▼
          ┌──────────────┐  ┌──────────┐  ┌──────────────────┐
          │COMPRESSED    │  │ PRUNED   │  │ CRYSTALLIZED     │
          │(level 1)     │  │(deleted) │  │(level 2, semantic)│
          │summary only   │  │          │  │permanent memory   │
          └──────────────┘  └──────────┘  └──────────────────┘
```

---

## Relationships to Other Nodes

| Other Node | Via Edge(s) | Meaning |
|---|---|---|
| `Concept` | `Recalls` + `AbstractsTo` | What knowledge this engram contributes to |
| `Concept` (indirectly) | `Concept → Defines → Identity` | How this engram shapes the AI's identity (2-hop traversal) |
| `Engram` (same type) | `Contradicts` | This engram conflicts with another about the same concept |
| `Engram` (same type) | `Reinforces` | This engram supports/confirms another |
| `Engram` (same type) | `TemporalSequence` | Chronological ordering between experiences |
| `Engram` (same type) | `SpawnedFrom` | Parent-child relationship after memory mitosis |

> **Note:** The `EngramVector` node type and `HasVector` edge type have been eliminated. The embedding is now a direct property on the Engram node.

---

## Typical Query Patterns

```rust
// Get engram by ID
g().n(NodeRef::var("engram_id"))

// Get engram's embedding (directly on the node — no traversal needed)
g().n(NodeRef::var("engram_id"))
    .property("embedding")

// Get all concepts this engram relates to
g().n(NodeRef::var("engram_id"))
    .out("Recalls")

// Get the full cognitive chain (engram → concepts → identity)
g().n(NodeRef::var("engram_id"))
    .out("Recalls")
    .out("Defines")

// Get engrams that contradict this one
g().n(NodeRef::var("engram_id"))
    .out("Contradicts")

// Find engrams by type (requires secondary index on engram_type)
g().n(NodeRef::var("engram_id"))
    .filter_node(PropertyFilter::property("engram_type").eq("episodic"))

// Vector search: find engrams semantically similar to a query embedding
// (requires vector index on Engram.embedding)
g().vector_search_with(
    "Engram",
    "embedding",
    query_embedding,
    NonZeroUsize::new(20).expect("non-zero"),
)

// Vector search with filter: find episodic engrams similar to query
g().vector_search_with(
    "Engram",
    "embedding",
    query_embedding,
    NonZeroUsize::new(20).expect("non-zero"),
).filter(PropertyFilter::property("engram_type").eq("episodic"))
```
