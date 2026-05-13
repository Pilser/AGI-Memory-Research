# Identity Queries — Personality Persistence

These queries manage the `Identity` subgraph — the stable, slow-changing personality traits that make the AI consistent over time.

---

## 1. GetAllIdentity

### What It Does
Returns all `Identity` trait nodes with their current values. Used by the Resonance Scorer to compute identity alignment (Factor 4) during retrieval.

### Parameters
None.

### Returns
    [{ trait, value, stability, last_updated }, ...]

Example:
    [
        { trait: "knows East African tech", value: 0.85, stability: 0.9, last_updated: "2026-04-01T..." },
        { trait: "prefers Rust for systems work", value: 0.6, stability: 0.4, last_updated: "2026-05-10T..." },
        { trait: "believes remote work is productive", value: 0.7, stability: 0.6, last_updated: "2026-03-15T..." }
    ]

### Used By
- `Pipeline.retrieve_memories()` — feeds identity traits into the resonance formula
- `Pipeline.consolidate()` — reads current identity state before applying updates
- Any code that needs to answer "Who am I?"

---

## 2. UpdateIdentityTrait

### What It Does
Updates an identity trait's `value` using the **weighted blending formula** that respects the trait's `stability`. High-stability traits resist change, low-stability traits adapt more readily.

### Parameters
| Param | Type | Purpose |
|---|---|---|
| `identity_id` | ID | Which trait to update |
| `new_value` | F32 | The new evidence/value (e.g., from a concept's confidence) |
| `stability` | F32 | The trait's current stability (passed to the formula) |
| `current_value` | F32 | The trait's current value (passed to the formula) |

### Formula (computed in HelixQL)
    new_value = ADD(
        MUL(current_value, stability),
        MUL(new_evidence, SUB(1.0, stability))
    )

    If stability = 0.9 (highly stable):
        new_value = current * 0.9 + evidence * 0.1   → 90% old, 10% new

    If stability = 0.3 (still forming):
        new_value = current * 0.3 + evidence * 0.7   → 30% old, 70% new

### Used By
- Identity crystallization (consolidation loop)
- When concepts that define an identity trait change confidence

### Rust Usage Pattern
    let traits = client.query("GetAllIdentity", &json!({})).await?;
    for t in &traits {
        let blended = t.value * t.stability + new_evidence * (1.0 - t.stability);
        client.query("UpdateIdentityTrait", &json!({
            "identity_id": t.id,
            "new_value": new_evidence,
            "stability": t.stability,
            "current_value": t.value
        })).await?;
    }

---

## Identity Crystallization Algorithm

Run during consolidation:

    For each concept:
        if concept.confidence > 0.7 AND concept.source_count > 20:
            existing_trait = find Identity where trait == concept.name
            if not existing_trait:
                CreateIdentity(
                    trait: concept.name,
                    value: concept.confidence,
                    stability: 0.5     // new trait, moderate stability
                )
                ConnectDefinition(concept_id, identity_id)
            else:
                // Existing trait — weighted update
                UpdateIdentityTrait(
                    identity_id: existing_trait.id,
                    new_value: concept.confidence,
                    stability: existing_trait.stability,
                    current_value: existing_trait.value
                )

    For each identity trait:
        if trait.last_updated is older than 3 months:
            // Trait has persisted → increase stability
            UpdateIdentityTrait(identity_id, same_value, stability + 0.05, same_value)

---

## Contradiction Protection

Before updating a high-stability identity trait, check for contradictions:

    If |new_evidence - current_value| > 0.5 AND stability > 0.8:
        DON'T update automatically
        Flag for contradiction resolution
        This prevents: one bad experience flipping a core belief

    If |new_evidence - current_value| > 0.5 AND stability <= 0.3:
        DO update (trait is still forming, strong evidence should shift it)
