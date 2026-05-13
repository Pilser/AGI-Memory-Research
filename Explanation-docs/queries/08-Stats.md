# Stats & Aggregation Queries — Memory Health Monitoring

These queries provide aggregate statistics about the memory system's state — how many engrams exist, how they're distributed by type, how many contradictions are active, etc. Used for monitoring, debugging, and dashboards.

---

## 1. CountEngramsByType

### What It Does
Groups engrams by their `engram_type` field and returns the count for each type. Uses HelixDB's `GROUP_BY` which returns count summaries only (no data objects — efficient).

### Parameters
None.

### Returns
    [
        { engram_type: "episodic", count: 3421 },
        { engram_type: "semantic", count: 487 },
        { engram_type: "procedural", count: 93 }
    ]

### Used By
- Dashboard/metrics — tracking memory composition over time
- Consolidation monitoring — seeing how many engrams have been promoted from episodic → semantic
- Health checks — are we accumulating too many uncompressed episodic engrams?

---

## 2. GetMemoryStats

### What It Does
Returns a comprehensive snapshot of the entire memory system: total engrams, active contradictions, total concepts, and total identity traits — all in one query.

### Parameters
None.

### Returns
    {
        total_engrams: 4001,
        contradiction_count: 23,
        concept_count: 847,
        identity_count: 35
    }

### Used By
- Dashboard — "Memory system status"
- Health monitoring — alert if contradiction_count spikes
- Ratio analysis — concept_count / total_engrams (should grow over time, meaning knowledge is consolidating)
- identity_count / concept_count (should converge, meaning knowledge is crystallizing into identity)

---

## Interpreting the Stats

| Ratio | Healthy Trend | Meaning |
|---|---|---|
| `concept_count / total_engrams` | Decreasing over time | Knowledge is consolidating — fewer concepts, more engrams per concept |
| `identity_count / concept_count` | Converging to ~0.05–0.10 | Only the strongest concepts crystallize into identity |
| `contradiction_count / total_engrams` | < 0.01 (1%) | Few unresolved contradictions |
| `semantic / episodic` ratio | Slowly increasing | Episodic memories are being promoted to semantic knowledge |
| `pruned_engrams` (derived) | Steady rate | Forgetting is working correctly |

---

## Additional Aggregation Possibilities

These aren't pre-written queries but can be added as the system grows:

    -- Average importance by engram type
    N<Engram>::GROUP_BY(engram_type, importance_score)

    -- Most recalled engrams (high activation_count)
    N<Engram>::ORDER<Desc>(_::{activation_count})::RANGE(0, 10)

    -- Concepts by depth in the SpawnedFrom hierarchy
    -- (requires recursive traversal in application code)

    -- Average emotional charge over time (trend analysis)
    -- (requires application code to bucket by time periods)

    -- Orphan concepts (no incoming Recalls edges)
    N<Concept>::WHERE(!EXISTS(_::In<Recalls>))
