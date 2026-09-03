# Identity Queries — Personality Persistence

These queries manage the `Identity` subgraph — the stable, slow-changing personality traits that make the AI consistent over time.

---

## 1. GetAllIdentity

### What It Does
Returns all `Identity` trait nodes with their current values. Used by the Resonance Scorer to compute identity alignment (Factor 4) during retrieval.

### Parameters
None.

### Returns
```json
{
    "traits": [
        { "$id": 1, "trait": "knows East African tech", "value": 0.85, "stability": 0.9, "last_updated": "2026-04-01T..." },
        { "$id": 2, "trait": "prefers Rust for systems work", "value": 0.6, "stability": 0.4, "last_updated": "2026-05-10T..." },
        { "$id": 3, "trait": "believes remote work is productive", "value": 0.7, "stability": 0.6, "last_updated": "2026-03-15T..." }
    ]
}
```

### Used By
- `Pipeline.retrieve_memories()` — feeds identity traits into the resonance formula
- `Pipeline.consolidate()` — reads current identity state before applying updates
- Any code that needs to answer "Who am I?"

### Query Implementation
```rust
use helix_db::dsl::prelude::*;

#[query]
fn get_all_identity() -> ReadBatch {
    read_batch()
        .var_as("traits", g()
            .n_with_label("Identity")
            .value_map(Some(vec!["$id", "trait", "value", "stability", "last_updated"])))
        .returning(["traits"])
}
```

### Typical Query Patterns
```rust
let request = get_all_identity();
let response = client.query(request).send().await?;
let traits: Vec<IdentityTrait> = serde_json::from_value(response["traits"].clone())?;
```

---

## 2. UpdateIdentityTrait

### What It Does
Updates an identity trait's `value` using the **weighted blending formula** that respects the trait's `stability`. High-stability traits resist change, low-stability traits adapt more readily.

### Parameters
| Param | Type | Purpose |
|---|---|---|
| `identity_id` | u64 | Which trait to update |
| `new_value` | f64 | The new evidence/value (e.g., from a concept's confidence) |
| `stability` | f64 | The trait's current stability (passed to the formula) |
| `current_value` | f64 | The trait's current value (passed to the formula) |

### Formula (computed in Rust)
```rust
// Weighted update: new = old * stability + evidence * (1 - stability)
let blended = current_value * stability + new_value * (1.0 - stability);

// If stability = 0.9 (highly stable):
//     blended = current * 0.9 + evidence * 0.1   → 90% old, 10% new
//
// If stability = 0.3 (still forming):
//     blended = current * 0.3 + evidence * 0.7   → 30% old, 70% new
```

### Used By
- Identity crystallization (consolidation loop)
- When concepts that define an identity trait change confidence

### Query Implementation
```rust
#[query]
fn update_identity_trait(
    identity_id: u64,
    new_value: f64,
    stability: f64,
    current_value: f64,
) -> WriteBatch {
    // Weighted update: new = old * stability + evidence * (1 - stability)
    let blended = current_value * stability + new_value * (1.0 - stability);

    write_batch()
        .var_as("updated", g()
            .n(NodeRef::id(identity_id))
            .set_property("value", blended)
            .set_property("last_updated", chrono::Utc::now().to_rfc3339()))
        .returning(["updated"])
}
```

### Typical Query Patterns
```rust
let traits_request = get_all_identity();
let traits_response = client.query(traits_request).send().await?;
let traits: Vec<IdentityTrait> = serde_json::from_value(traits_response["traits"].clone())?;

for t in &traits {
    let request = update_identity_trait(
        t.id,
        new_evidence,       // from concept confidence
        t.stability,        // current stability
        t.value,            // current value
    );
    client.query(request).send().await?;
}
```

---

## Identity Crystallization Algorithm

Run during consolidation:

```rust
// For each concept:
for concept in concepts {
    if concept.confidence > 0.7 && concept.source_count > 20 {
        // Check if identity trait already exists
        let existing = find_identity_by_trait(&concept.name);
        
        match existing {
            None => {
                // Create new identity trait
                create_identity(
                    concept.name,
                    concept.confidence,
                    0.5,  // new trait, moderate stability
                ).await?;
                connect_definition(concept.id, identity_id).await?;
            }
            Some(trait_node) => {
                // Existing trait — weighted update
                update_identity_trait(
                    trait_node.id,
                    concept.confidence,
                    trait_node.stability,
                    trait_node.value,
                ).await?;
            }
        }
    }
}

// For each identity trait:
for trait in identity_traits {
    if trait.last_updated older than 3 months {
        // Trait has persisted → increase stability
        update_identity_trait(
            trait.id,
            trait.value,
            trait.stability + 0.05,
            trait.value,
        ).await?;
    }
}
```

---

## Contradiction Protection

Before updating a high-stability identity trait, check for contradictions:

```rust
if (new_evidence - current_value).abs() > 0.5 && stability > 0.8 {
    // DON'T update automatically
    // Flag for contradiction resolution
    // This prevents: one bad experience flipping a core belief
} else if (new_evidence - current_value).abs() > 0.5 && stability <= 0.3 {
    // DO update (trait is still forming, strong evidence should shift it)
    update_identity_trait(identity_id, new_evidence, stability, current_value).await?;
}
```
