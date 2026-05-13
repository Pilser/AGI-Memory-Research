# Temporal Queries — Timeline & Chronology

These queries manage the temporal dimension of memory — ordering experiences by when they occurred, navigating forward and backward in the AI's personal timeline.

---

## 1. GetTimeline

### What It Does
Returns a paginated, chronologically ordered list of engrams — the AI's "memory timeline." Sorted by `timestamp` descending (newest first). Supports offset-based pagination.

### Parameters
| Param | Type | Purpose |
|---|---|---|
| `limit` | I64 | How many engrams to return (page size) |
| `offset` | I64 | Starting position for pagination |

### Returns
    [{ ID, episode_raw, engram_type, emotional_charge, timestamp }, ...]

Ordered newest → oldest.

### Used By
- Timeline browsing — "What did I experience in the last week?"
- Session context — loading recent memories to understand current context
- Debugging memory chronology

### Pagination Example
    Page 1: GetTimeline(limit=10, offset=0)  → memories 1-10 (newest)
    Page 2: GetTimeline(limit=10, offset=10) → memories 11-20
    Page 3: GetTimeline(limit=10, offset=20) → memories 21-30

---

## 2. GetTemporalChain

### What It Does
For a specific engram, returns both the previous engram (what happened before) and the next engram (what happened after). This enables navigating the timeline one step at a time in either direction.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | ID |

### Returns
    {
        previous: [Engram, ...],   // engrams that happened before this one (In<TemporalSequence>)
        next: [Engram, ...]        // engrams that happened after this one (Out<TemporalSequence>)
    }

### Used By
- Context expansion — "What led up to this memory?"
- Causal reasoning — understanding sequences of events
- Narrative construction — building coherent stories from temporal chains

### Important
`TemporalSequence` edges must be created at ingestion time for this to work. The ingestion pipeline links each new engram to the most recent engram before it via `ConnectTemporal`.

---

## Why Temporal Edges Matter

Without temporal ordering, the AI can't:
- Tell a coherent story about what happened when
- Understand cause and effect ("X happened, then Y happened")
- Answer time-based questions ("What did I learn about databases before I learned about Postgres?")
- Detect changes over time ("My opinion shifted from positive to negative between these two events")
