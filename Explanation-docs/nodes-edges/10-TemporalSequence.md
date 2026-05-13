# E::TemporalSequence — Engram → Engram

## What It Is

Links two engrams in **chronological order** — "this happened, then that happened." Builds the timeline of the AI's experiences. Unlike other edges that are about semantic relationships, this edge is purely temporal: it orders memories by when they occurred.

---

## Schema

    E::TemporalSequence {
        From: Engram,
        To:   Engram
    }

---

## Direction

    Earlier Engram ──TemporalSequence──→ Later Engram

    From: The engram that happened first
    To:   The engram that happened later

---

## When It's Created

| Trigger | Timing |
|---|---|
| Engram ingestion (automatic) | The new engram gets a TemporalSequence edge from the most recent engram before it |
| Batch ordering | During consolidation, all engrams can be sorted by timestamp and linked |

---

## How It's Used

| Use Case | Query |
|---|---|
| **Get timeline** | `GetTimeline(limit, offset)` — orders by timestamp DESC |
| **What happened before this?** | `N<Engram>(id)::In<TemporalSequence>` |
| **What happened after this?** | `N<Engram>(id)::Out<TemporalSequence>` |
| **Trace experience chains** | `N<Engram>(id)::Out<TemporalSequence>::Out<TemporalSequence>::Out<TemporalSequence>` |

---

## Why It Matters

Temporal ordering enables:
- **Causal reasoning** — "I learned X before Y, so X influenced Y"
- **Timeline queries** — "What did I experience last Tuesday?"
- **Recency context** — Understanding what sequence of events led to a conclusion
- **Narrative construction** — The AI can tell coherent stories about its experiences
