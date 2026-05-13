# E::AbstractsTo — Engram → Concept

## What It Is

Links an engram to the broader concepts it **abstracts into**. While `Recalls` means "this engram mentions this concept," `AbstractsTo` means "this engram contributes to/distills into this concept." It represents the abstraction hierarchy: raw experiences feed into general knowledge.

---

## Schema

    E::AbstractsTo {
        From: Engram,
        To:   Concept
    }

---

## Direction

    Engram ──AbstractsTo──→ Concept

    From: Engram    (the experience that feeds the concept)
    To:   Concept   (the abstraction that the experience supports)

---

## When It's Created

| Trigger | Query | Timing |
|---|---|---|
| Concept extraction at ingestion | `ConnectAbstraction(engram_id, concept_id)` | Paired with every Recalls edge |

---

## Recalls vs AbstractsTo

| Aspect | Recalls | AbstractsTo |
|---|---|---|
| **Meaning** | "This engram mentions/talks about this concept" | "This engram's knowledge feeds into this concept" |
| **Granularity** | Usually same level — engram about X → concept X | Can be one level up — engram about X → broader concept Y |
| **Usage** | Counting source_count, contradiction detection, direct retrieval | Building abstraction hierarchy, identity alignment |
| **Mitosis reassignment** | Yes — both edges reassigned | Yes — both edges reassigned |

---

## Typical Query Patterns

    -- Get concepts abstracted from an engram
    N<Engram>(id)::Out<AbstractsTo>

    -- Get engrams that abstract to a concept
    N<Concept>(id)::In<AbstractsTo>

    -- Find the full abstraction chain
    N<Engram>(id)::Out<AbstractsTo>::Out<Defines>  -- engram → concept → identity
