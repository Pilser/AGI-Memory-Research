# N::Engram — The Cognitive Memory Unit

## What It Is

The engram is the **fundamental unit of memory** in MNEMOS. It represents a single discrete experience, thought, fact, or observation — analogous to what neuroscience calls an "engram" (the physical trace of a memory in the brain). Every interaction the AI has that's worth remembering becomes an Engram node.

---

## Schema

```hql
N::Engram {
    episode_raw:       String,     // The full raw text of the memory
    emotional_charge:  F32,        // -1.0 (negative) to +1.0 (positive)
    importance_score:  F32,        // 0.0 (trivial) to 1.0 (critical)
    decay_rate:        F32,        // Ebbinghaus forgetting speed
    activation_count:  U32,        // How many times this engram was recalled
    timestamp:         Date,       // When the memory was formed
    engram_type:       String,     // "episodic" | "semantic" | "procedural"
    compression_level: U8,         // 0=raw, 1=compressed (summary only), 2=crystallized
    contradiction_flag: Boolean    // True if this engram contradicts another
}
```

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

---

## Outgoing Edges (What an Engram Can Point To)

```
Engram ──HasVector────────→ EngramVector    (its embedding)
Engram ──Recalls──────────→ Concept         (what concepts this memory relates to)
Engram ──AbstractsTo──────→ Concept         (what concepts this memory feeds into)
Engram ──Contradicts──────→ Engram          (this memory contradicts another)
Engram ──Reinforces───────→ Engram          (this memory reinforces another)
Engram ──TemporalSequence─→ Engram          (this memory happened before that one)
Engram ──SpawnedFrom──────→ Engram          (this is a child of parent memory after mitosis)
```

## Incoming Edges (What Points to an Engram)

```
EngramVector ──HasVector────────← Engram    (an embedding represents this engram)
Concept      ──Recalls──────────← Engram    (this concept is recalled by engrams)
Concept      ──AbstractsTo──────← Engram    (this concept is built from engrams)
Engram       ──Contradicts──────← Engram    (another memory contradicts this one)
Engram       ──Reinforces───────← Engram    (another memory reinforces this one)
Engram       ──TemporalSequence─← Engram    (a memory happened before this one)
Engram       ──SpawnedFrom──────← Engram    (another memory spawned this one via mitosis)
```

---

## What It's Used For

| Use Case | How |
|---|---|
| **Memory storage** | `CreateEngram` / `CreateFullEngram` creates the node |
| **Memory retrieval** | `SearchEngramsByText` → `SearchV` finds relevant engrams by embedding |
| **Memory decay** | Consolidation loop reads `decay_rate`, `timestamp`, `activation_count` to compute retention |
| **Memory pruning** | `DROP N<Engram>(id)` deletes forgotten engrams |
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
| `EngramVector` | `HasVector` | This engram's semantic embedding (used for vector search) |
| `Concept` | `Recalls` + `AbstractsTo` | What knowledge this engram contributes to |
| `Concept` (indirectly) | `Concept → Defines → Identity` | How this engram shapes the AI's identity (2-hop traversal) |
| `Engram` (same type) | `Contradicts` | This engram conflicts with another about the same concept |
| `Engram` (same type) | `Reinforces` | This engram supports/confirms another |
| `Engram` (same type) | `TemporalSequence` | Chronological ordering between experiences |
| `Engram` (same type) | `SpawnedFrom` | Parent-child relationship after memory mitosis |

---

## Typical Query Patterns Involving Engram

```
-- Get engram by ID
N<Engram>(engram_id)

-- Get engram with its embedding
N<Engram>(engram_id)::Out<HasVector>::FIRST

-- Get all concepts this engram relates to
N<Engram>(engram_id)::Out<Recalls>

-- Get the full cognitive chain (engram → concepts → identity)
N<Engram>(engram_id)::Out<Recalls>::Out<Defines>

-- Get engrams that contradict this one
N<Engram>(engram_id)::Out<Contradicts>

-- Find engrams by type
N<Engram>::WHERE(_::{engram_type}::EQ("episodic"))

-- Count engrams by type
N<Engram>::GROUP_BY(engram_type)
```
