# E::Defines — Concept → Identity

## What It Is

Links a concept to an identity trait that it **defines** or shapes. This is the bridge from knowledge ("I know about X") to self ("I am someone who knows about X"). When a concept becomes strong enough (high confidence, high source_count), it crystallizes into an identity trait via a `Defines` edge.

---

## Schema

```rust
g().n(NodeRef::var("concept_id")).add_e("Defines", NodeRef::var("identity_id"), vec![])
```

---

## Direction

```
Concept ──Defines──→ Identity

From: Concept    (e.g., "East African tech ecosystem")
To:   Identity   (e.g., "knows East African tech")
```

---

## When It's Created

| Trigger | Timing |
|---|---|
| Identity crystallization | During consolidation, when a concept's `confidence > 0.7` AND `source_count > 20` |
| Manual identity seeding | User or system explicitly defines identity traits from concepts |

---

## How It's Used

| Use Case | Query |
|---|---|
| **What identity traits does a concept define?** | Forward traversal from Concept |
| **What concepts define an identity trait?** | Backward traversal from Identity |
| **Engram → Identity (full chain)** | Engram → Recalls → Defines → Identity |
| **Identity alignment scoring** | Walk chain, compute cosine sim between engram embedding and identity trait embeddings |

---

## The Full Cognitive Chain

```
Engram (raw memory)
   │
   ├── Recalls/AbstractsTo → Concept (distilled knowledge)
   │                              │
   │                              └── Defines → Identity (stable self-model)
   │
   └── Contradicts/Reinforces → Other Engrams (relationship to other memories)
```

This 3-layer architecture is what makes MNEMOS different from RAG:

| Layer | RAG Equivalent | MNEMOS |
|---|---|---|
| Engram | Document chunk | Raw experience with metadata |
| Concept | (nothing) | Abstracted knowledge that grows over time |
| Identity | (nothing) | Stable personality that resists rapid change |

---

## Identity Trait Weight from Concepts

The `value` of an identity trait is derived from the concepts that define it:

```
For Identity{trait: "knows databases"}:
  → In<Defines> → [Concept{name: "PostgreSQL", confidence: 0.9, source_count: 45},
                    Concept{name: "MongoDB", confidence: 0.7, source_count: 20},
                    Concept{name: "Redis", confidence: 0.6, source_count: 15}]

Identity.value = weighted_avg([0.9, 0.7, 0.6], [45, 20, 15])
               = (0.9*45 + 0.7*20 + 0.6*15) / 80
               = 0.79
```

The more concepts define a trait, and the stronger those concepts are, the higher the trait's value.

---

## Typical Query Patterns

```rust
// Create a Defines edge
g().n(NodeRef::var("concept_id"))
    .add_e("Defines", NodeRef::var("identity_id"), vec![])

// What identity traits does a concept define?
g().n(NodeRef::var("concept_id"))
    .out("Defines")

// What concepts define an identity trait?
g().n(NodeRef::var("identity_id"))
    .in("Defines")

// Engram → Identity (full chain)
g().n(NodeRef::var("engram_id"))
    .out("Recalls")
    .out("Defines")
```
