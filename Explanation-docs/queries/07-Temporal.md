# Temporal Queries — Timeline & Chronology

These queries manage the temporal dimension of memory — ordering experiences by when they occurred, navigating forward and backward in the AI's personal timeline.

---

## 1. GetTimeline

### What It Does
Returns a paginated, chronologically ordered list of engrams — the AI's "memory timeline." Sorted by `timestamp` descending (newest first). Supports offset-based pagination.

### Parameters
| Param | Type | Purpose |
|---|---|---|
| `limit` | i64 | How many engrams to return (page size) |
| `offset` | i64 | Starting position for pagination |

### Returns
```json
{
    "engrams": [
        { "$id": 99, "episode_raw": "Latest memory...", "engram_type": "episodic", "emotional_charge": 0.5, "timestamp": "2026-05-15T10:00:00Z" },
        { "$id": 98, "episode_raw": "Previous memory...", "engram_type": "episodic", "emotional_charge": -0.2, "timestamp": "2026-05-14T09:00:00Z" }
    ]
}
```

Ordered newest → oldest.

### Used By
- Timeline browsing — "What did I experience in the last week?"
- Session context — loading recent memories to understand current context
- Debugging memory chronology

### Pagination Example
```rust
// Page 1: memories 1-10 (newest)
let request = get_timeline(10, 0);
let page1 = client.query(request).send().await?;

// Page 2: memories 11-20
let request = get_timeline(10, 10);
let page2 = client.query(request).send().await?;

// Page 3: memories 21-30
let request = get_timeline(10, 20);
let page3 = client.query(request).send().await?;
```

### Query Implementation
```rust
use helix_db::dsl::prelude::*;

#[query]
fn get_timeline(
    limit: i64,
    offset: i64,
) -> ReadBatch {
    read_batch()
        .var_as("engrams", g()
            .n_with_label("Engram")
            // Note: ORDER BY support in Rust DSL may be limited
            // Alternative: fetch all and sort client-side
            .limit(limit + offset)
            .value_map(Some(vec![
                "$id", "episode_raw", "engram_type", "emotional_charge", "timestamp",
            ])))
        .returning(["engrams"])
}
```

**Note on ordering:** The old approach used `ORDER<Desc>(_::{timestamp})` and `RANGE(offset, limit)`. The new DSL's Rust SDK may not expose direct ORDER BY / RANGE operations. The workaround is to fetch with a limit and handle pagination client-side, or use the JSON AST directly for advanced operations.

---

## 2. GetTemporalChain

### What It Does
For a specific engram, returns both the previous engram (what happened before) and the next engram (what happened after). This enables navigating the timeline one step at a time in either direction.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | u64 |

### Returns
```json
{
    "next": [{ "$id": 43, "episode_raw": "What happened after...", "timestamp": "2026-05-11T..." }],
    "prev": [{ "$id": 41, "episode_raw": "What happened before...", "timestamp": "2026-05-09T..." }]
}
```

### Used By
- Context expansion — "What led up to this memory?"
- Causal reasoning — understanding sequences of events
- Narrative construction — building coherent stories from temporal chains

### Important
`TemporalSequence` edges must be created at ingestion time for this to work. The ingestion pipeline links each new engram to the most recent engram before it via `ConnectTemporal`.

### Query Implementation
```rust
#[query]
fn get_temporal_chain(engram_id: u64) -> ReadBatch {
    read_batch()
        .var_as("next", g()
            .n(NodeRef::id(engram_id))
            .out(Some("TemporalSequence"))
            .value_map(Some(vec!["$id", "episode_raw", "timestamp"])))
        .var_as("prev", g()
            .n(NodeRef::id(engram_id))
            .in_(Some("TemporalSequence"))
            .value_map(Some(vec!["$id", "episode_raw", "timestamp"])))
        .returning(["next", "prev"])
}
```

### Typical Query Patterns
```rust
// Navigate forward in time from a specific memory
let request = get_temporal_chain(engram_id);
let response = client.query(request).send().await?;

let next_memories: Vec<TemporalNode> = serde_json::from_value(response["next"].clone())?;
let prev_memories: Vec<TemporalNode> = serde_json::from_value(response["prev"].clone())?;

println!("This memory was preceded by: {:?}", prev_memories);
println!("This memory was followed by: {:?}", next_memories);
```

---

## Why Temporal Edges Matter

Without temporal ordering, the AI can't:
- Tell a coherent story about what happened when
- Understand cause and effect ("X happened, then Y happened")
- Answer time-based questions ("What did I learn about databases before I learned about Postgres?")
- Detect changes over time ("My opinion shifted from positive to negative between these two events")
