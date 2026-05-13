# E::Reinforces — Engram → Engram

## What It Is

Links two engrams where one **reinforces** or supports the other. This is the opposite of `Contradicts`. When two engrams about the same concept have the same emotional valence and semantically similar content, one reinforces the other — strengthening the concept's confidence.

---

## Schema

    E::Reinforces {
        From: Engram,
        To:   Engram
    }

---

## Direction

    Engram A ──Reinforces──→ Engram B

    From: The reinforcing engram (adds support)
    To:   The reinforced engram (receives support)

---

## When It's Created

| Trigger | Timing |
|---|---|
| Concept extraction finds multiple engrams with same emotional sign about same concept | During consolidation or ingestion |
| Two engrams about the same concept have cosine similarity > 0.7 AND same emotional sign | Batch detection |

Reinforcement detection uses the same mechanics as contradiction detection but with the opposite condition: same sign instead of opposite sign.

---

## Downstream Effects

| Effect | How |
|---|---|
| **Concept confidence boost** | Concepts that accumulate many reinforcing engrams increase in confidence |
| **Identity stability** | Reinforced concepts are more likely to crystallize into identity traits |
| **Memory persistence** | Reinforced engrams may get their importance_score boosted (they're confirmed by multiple experiences) |

---

## Difference from Recalls

- `Recalls` says: "engram mentions concept X"
- `Reinforces` says: "engram A and engram B are saying the same thing about X"

Two engrams can both have `Recalls` to the same concept without reinforcing each other (e.g., one about PostgreSQL performance, one about PostgreSQL installation — same concept, different sub-topics). `Reinforces` requires semantic similarity between the engrams themselves, not just shared concept membership.
