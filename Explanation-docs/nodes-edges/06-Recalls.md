# E::Recalls — Engram → Concept

## What It Is

Links an engram to the concepts it **recalls** (references, mentions, or discusses). This is the primary connection from raw experience to abstracted knowledge. When the AI remembers something about "PostgreSQL," it's because engrams have `Recalls` edges pointing to the "PostgreSQL" concept.

---

## Schema

    E::Recalls {
        From: Engram,
        To:   Concept
    }

No properties — a structural link.

---

## Direction

    Engram ──Recalls──→ Concept

    From: Engram    (the memory that mentions the concept)
    To:   Concept   (the abstracted knowledge piece)

---

## When It's Created

| Trigger | Query | Timing |
|---|---|---|
| Concept extraction at ingestion | `ConnectRecalls(engram_id, concept_id)` | After concept is created/found, one edge per concept |

---

## How It's Used

| Use Case | Query | Direction |
|---|---|---|
| **What concepts does this engram mention?** | `N<Engram>(id)::Out<Recalls>` | Forward |
| **What engrams mention this concept?** | `N<Concept>(id)::In<Recalls>` | Backward |
| **Cognitive chain to identity** | `N<Engram>(id)::Out<Recalls>::Out<Defines>` | Forward → Forward |
| **Contradiction candidates** | `N<Concept>(id)::In<Recalls>` → compare emotional_charges | Backward |
| **Mitosis clustering** | `N<Concept>(id)::In<Recalls>` → get engram embeddings | Backward |
| **Count engrams per concept** | `N<Concept>(id)::In<Recalls>::COUNT` | Backward |

---

## Recalls vs AbstractsTo

Both edges go from Engram to Concept, but they have different semantic meanings:

| Edge | Meaning | Example |
|---|---|---|
| `Recalls` | "This engram mentions/recalls this concept" | "I read about Uganda ICT Hub today" → Recalls → "Uganda ICT Hub" |
| `AbstractsTo` | "This engram contributes to/abstracts into this concept" | Same engram → AbstractsTo → "East African tech ecosystem" (a broader abstraction) |

In practice, every `Recalls` edge is usually paired with an `AbstractsTo` edge, since mentioning a concept also contributes to it. But the distinction becomes useful when engrams contribute to concepts they don't explicitly name (e.g., an engram about "Nairobi VC" abstracting to "East African investment trends").

---

## Lifecycle

| Event | Effect |
|---|---|
| Engram created | Concept Extractor creates Recalls edges for each extracted concept |
| Mitosis | Edges are reassigned from parent concept to child concepts for clustered engrams |
| Engram pruned | Edge cascade-deleted |
| Concept pruned | Edge cascade-deleted |

---

## Relationship to source_count

The `source_count` on a Concept node should equal the number of incoming `Recalls` edges. The `UpdateConceptSourceCount` query syncs this:

    QUERY UpdateConceptSourceCount(concept_id: ID) =>
        concept <- N<Concept>(concept_id)
        count <- concept::In<Recalls>::COUNT
        updated <- concept::UpsertN({source_count: count})
        RETURN updated
