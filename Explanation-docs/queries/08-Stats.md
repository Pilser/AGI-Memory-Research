# Stats & Aggregation Queries — Memory Health Monitoring

These queries provide aggregate statistics about the memory system's state — how many engrams exist, how they're distributed by type, how many contradictions are active, etc. Used for monitoring, debugging, and dashboards.

---

## 1. CountEngramsByType

### What It Does
Groups engrams by their `engram_type` field and returns the count for each type. Uses HelixDB's `count()` which returns count summaries only (no data objects — efficient).

### Parameters
None.

### Returns
```json
{
    "counts": [
        { "engram_type": "episodic", "count": 3421 },
        { "engram_type": "semantic", "count": 487 },
        { "engram_type": "procedural", "count": 93 }
    ]
}
```

### Used By
- Dashboard/metrics — tracking memory composition over time
- Consolidation monitoring — seeing how many engrams have been promoted from episodic → semantic
- Health checks — are we accumulating too many uncompressed episodic engrams?

### Query Implementation
```rust
use helix_db::dsl::prelude::*;

#[query]
fn count_engrams_by_type() -> ReadBatch {
    read_batch()
        .var_as("counts", g()
            .n_with_label("Engram")
            .count())
        .returning(["counts"])
}
```

### Typical Query Patterns
```rust
let request = count_engrams_by_type();
let response = client.query(request).send().await?;
let counts: Vec<TypeCount> = serde_json::from_value(response["counts"].clone())?;

for c in &counts {
    println!("{}: {}", c.engram_type, c.count);
}
```

---

## 2. GetMemoryStats

### What It Does
Returns a comprehensive snapshot of the entire memory system: total engrams, active contradictions, total concepts, and total identity traits — all in one query.

### Parameters
None.

### Returns
```json
{
    "total": 4001,
    "contradictions": 23,
    "concepts": 847,
    "identities": 35
}
```

### Used By
- Dashboard — "Memory system status"
- Health monitoring — alert if contradiction_count spikes
- Ratio analysis — concept_count / total_engrams (should grow over time, meaning knowledge is consolidating)
- identity_count / concept_count (should converge, meaning knowledge is crystallizing into identity)

### Query Implementation
```rust
#[query]
fn get_memory_stats() -> ReadBatch {
    read_batch()
        .var_as("total", g()
            .n_with_label("Engram")
            .count())
        .var_as("contradictions", g()
            .n_with_label_where("Engram", SourcePredicate::eq("contradiction_flag", true))
            .count())
        .var_as("concepts", g()
            .n_with_label("Concept")
            .count())
        .var_as("identities", g()
            .n_with_label("Identity")
            .count())
        .returning(["total", "contradictions", "concepts", "identities"])
}
```

### Typical Query Patterns
```rust
let request = get_memory_stats();
let response = client.query(request).send().await?;

let total: i64 = serde_json::from_value(response["total"].clone())?;
let contradictions: i64 = serde_json::from_value(response["contradictions"].clone())?;
let concepts: i64 = serde_json::from_value(response["concepts"].clone())?;
let identities: i64 = serde_json::from_value(response["identities"].clone())?;

println!("Memory Stats:");
println!("  Total engrams: {}", total);
println!("  Contradictions: {} ({:.1}%)", contradictions, contradictions as f64 / total as f64 * 100.0);
println!("  Concepts: {}", concepts);
println!("  Identity traits: {}", identities);
println!("  Concept/Engram ratio: {:.3}", concepts as f64 / total as f64);
println!("  Identity/Concept ratio: {:.3}", identities as f64 / concepts as f64);
```

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

```rust
// Average importance by engram type
// (fetch all and aggregate client-side)

// Most recalled engrams (high activation_count)
// (fetch all, sort by activation_count, take top 10)

// Concepts by depth in the SpawnedFrom hierarchy
// (requires recursive traversal in application code)

// Average emotional charge over time (trend analysis)
// (requires application code to bucket by time periods)

// Orphan concepts (no incoming Recalls edges)
// (fetch concepts, filter those with no incoming Recalls)
```
