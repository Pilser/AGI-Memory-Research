# N::Concept — Abstracted Knowledge Unit

## What It Is

A **Concept** is an abstracted piece of knowledge extracted from one or more engrams. While an engram is a raw memory ("I saw X happen"), a concept is the distilled understanding ("X exists" or "X is true"). Concepts are the bridge between episodic experience and stable knowledge — they grow stronger as more engrams feed into them.

---

## Schema

    N::Concept {
        name:              String,      // Human-readable concept name
        confidence:        F32,         // How strongly the system believes this concept (0.0–1.0)
        formation_date:    Date,        // When this concept was first created
        source_count:      U32          // How many engrams feed into this concept
    }

---

## Fields Explained

| Field | Set By | Lifecycle |
|---|---|---|
| `name` | Concept Extractor (ML) | Set at creation. Should be indexed for exact lookup. Used to deduplicate concepts. |
| `confidence` | Concept Extractor (ML) | Initial confidence from extraction. Can be boosted over time as more engrams reinforce it or lowered when contradictions emerge. |
| `formation_date` | System (`NOW`) | Set at creation. Used to track when the AI "learned" this concept. |
| `source_count` | `UpdateConceptSourceCount` query | Incremented every time a new engram links to this concept. High source_count → concept is well-supported. Triggers mitosis when above threshold. |

---

## Outgoing Edges (What a Concept Can Point To)

    Concept ──Defines────────→ Identity      (this concept shapes an identity trait)
    Concept ──SpawnedFrom────← Concept       (parent concept spawned this child via mitosis — incoming)

## Incoming Edges (What Points to a Concept)

    Engram  ──Recalls────────→← Concept      (engrams recall this concept)
    Engram  ──AbstractsTo────→← Concept      (engrams feed into this concept)
    Concept ──SpawnedFrom────→ Concept       (this is a child of a parent concept)

---

## What It's Used For

| Use Case | How |
|---|---|
| **Knowledge extraction** | Concept Extractor creates concepts from engram text |
| **Deduplication** | `GetConceptByName` checks if a concept already exists before creating a duplicate |
| **Knowledge strength** | `source_count` tracks how many engrams support a concept — high count = well-established knowledge |
| **Memory mitosis trigger** | When `source_count` exceeds threshold, the concept splits into specialized children |
| **Identity formation** | Concepts with high confidence and source_count become identity traits via `Defines` edges |
| **Contradiction grouping** | Engrams about the same concept are compared for contradictory emotional charges |
| **Semantic browsing** | Users or AI agents can query concepts by name to explore the knowledge graph |

---

## Lifecycle

    1. Engram ingested → Concept Extractor extracts concepts from text
    2. For each concept: check GetConceptByName(name) → exists?
       YES → reuse existing concept, increment source_count
       NO  → create new Concept node with source_count=1

    3. As more engrams arrive:
       source_count grows → confidence increases (more evidence)
       source_count exceeds threshold → MITOSIS triggers → child concepts spawn

    4. If contradictions are detected among engrams of this concept:
       confidence may decrease
       concept may split to resolve the contradiction

    5. If source_count stays at 1 and no new engrams link → concept atrophies
       (no explicit prune — orphan concepts just have no incoming edges)

---

## Concept Formation Example

    Engram 1: "The Uganda ICT Hub in Kampala is attracting VC"
      → extracts concept: "Uganda ICT Hub" (source_count = 1)

    Engram 2: "Visited Uganda ICT Hub, they have 50+ startups in residence"
      → same concept found → source_count = 2

    Engram 3: "Uganda ICT Hub just raised a $5M expansion fund"
      → source_count = 3

    ...after 47 more engrams about Uganda ICT Hub...

    source_count = 50 → ABOVE THRESHOLD → Mitosis triggers

    Clustering of 50 engram embeddings reveals 3 clusters:
      → Child: "Uganda ICT Hub funding and investment"  (source_count = 22)
      → Child: "Uganda ICT Hub startups and companies"   (source_count = 18)
      → Child: "Uganda ICT Hub infrastructure"            (source_count = 10)

    Parent "Uganda ICT Hub" stays as general category → source_count adjusted

---

## Relationships to Other Nodes

| Other Node | Via Edge(s) | Meaning |
|---|---|---|
| `Engram` | `Recalls` (incoming) | Engrams that reference this concept |
| `Engram` | `AbstractsTo` (incoming) | Engrams that contribute to/distill into this concept |
| `Identity` | `Defines` (outgoing) | This concept defines or shapes an identity trait |
| `Concept` | `SpawnedFrom` (incoming → parent, outgoing → children) | Knowledge hierarchy from general to specific |
| `Concept` | `Defines → Identity` (2-hop) | How concepts shape the AI's self-model |

---

## Typical Query Patterns

    -- Create concept
    AddN<Concept>({name: name, confidence: conf, formation_date: NOW, source_count: 1})

    -- Find concept by name (requires INDEX on name)
    N<Concept>({name: name})

    -- Get engrams that recall this concept
    N<Concept>(concept_id)::In<Recalls>

    -- Get identity traits this concept feeds into
    N<Concept>(concept_id)::Out<Defines>

    -- Get children spawned from this concept (mitosis)
    N<Concept>(concept_id)::In<SpawnedFrom>

    -- Get parent concept (what this was spawned from)
    N<Concept>(concept_id)::Out<SpawnedFrom>

    -- Count engrams per concept
    N<Concept>(concept_id)::In<Recalls>::COUNT

    -- Find concepts ready for mitosis
    N<Concept>::WHERE(_::{source_count}::GTE(min_engram_count))
