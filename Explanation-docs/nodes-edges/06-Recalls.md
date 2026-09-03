# E::Recalls — Engram → Concept

## What It Is

Links an engram to the concepts it **recalls** (references, mentions, or discusses). This is the primary connection from raw experience to abstracted knowledge. When the AI remembers something about "PostgreSQL," it's because engrams have `Recalls` edges pointing to the "PostgreSQL" concept.

---

## Schema

### Base Schema (Structural Link)

```rust
g().n(NodeRef::var("engram_id")).add_e("Recalls", NodeRef::var("concept_id"), vec![])
```

### Optional Enhancement: Edge Embedding

> **This is an OPTIONAL enhancement that improves the Stimulation Layer without degrading anything.**
> Store a relationship embedding on each Recalls edge to enable dynamic edge weights.

```rust
g().n(NodeRef::var("engram_id")).add_e("Recalls", NodeRef::var("concept_id"), vec![
    ("relationship_embedding", edge_embedding),  // Vec<f32> — how this engram relates to this concept
    ("strength", 0.85),                          // F64 — optional strength indicator
])
```

**How it works:**
- When creating a Recalls edge, compute a "relationship embedding" that captures HOW this engram relates to this concept
- This is different from the engram's own embedding (which captures WHAT the engram is about)
- Example: An engram about "Uganda ICT hub" might relate to concept "infrastructure sovereignty" through a POLITICAL angle or TECHNICAL angle

**Benefits (no degradation):**
- **Stimulation Layer:** Enables dynamic edge weights — stronger relationships transmit more activation (more biologically plausible)
- **Memory Mitosis:** Can cluster by relationship embeddings for more nuanced concept formation
- **CRR:** Optional combined scoring (engram similarity × relationship strength)

**Cost:** Extra storage (one embedding per edge) and computation (compute at edge creation)

---

## Direction

```
Engram ──Recalls──→ Concept

From: Engram    (the memory that mentions the concept)
To:   Concept   (the abstracted knowledge piece)
```

---

## When It's Created

| Trigger | Query | Timing |
|---|---|---|
| Concept extraction at ingestion | `ConnectRecalls(engram_id, concept_id)` | After concept is created/found, one edge per concept |

---

## How It's Used

| Use Case | Query | Direction |
|---|---|---|
| **What concepts does this engram mention?** | Forward traversal from Engram | Forward |
| **What engrams mention this concept?** | Backward traversal from Concept | Backward |
| **Cognitive chain to identity** | Engram → Recalls → Defines → Identity | Forward → Forward |
| **Contradiction candidates** | Get engrams recalling same concept → compare emotional_charges | Backward |
| **Mitosis clustering** | Get engrams recalling a concept → cluster their embeddings | Backward |
| **Count engrams per concept** | Count incoming Recalls edges | Backward |

---

## Recalls vs AbstractsTo

Both edges go from Engram to Concept, but they have different semantic meanings:

| Edge | Meaning | Example |
|---|---|---|
| `Recalls` | "This engram mentions/recalls this concept" | "I read about Uganda ICT Hub today" → Recalls → "Uganda ICT Hub" |
| `AbstractsTo` | "This engram contributes to/abstracts into this concept" | Same engram → AbstractsTo → "East African tech ecosystem" (a broader abstraction) |

In practice, every `Recalls` edge is usually paired with an `AbstractsTo` edge, since mentioning a concept also contributes to it. But the distinction becomes useful when engrams contribute to concepts they don't explicitly name (e.g., an engram about "Nairobi VC" abstracting to "East African investment trends").

---

## Lifecycle

| Event | Effect |
|---|---|
| Engram created | Concept Extractor creates Recalls edges for each extracted concept |
| Mitosis | Edges are reassigned from parent concept to child concepts for clustered engrams |
| Engram pruned | Edge cascade-deleted |
| Concept pruned | Edge cascade-deleted |

---

## Relationship to source_count

The `source_count` on a Concept node should equal the number of incoming `Recalls` edges. The `UpdateConceptSourceCount` query syncs this:

```rust
// UpdateConceptSourceCount — sync source_count with actual edge count
let request = write_batch()
    .var_as("concept", g().n(NodeRef::var("concept_id")))
    .var_as("count", g().n(NodeRef::var("concept_id")).in("Recalls").count())
    .var_as("updated", g().n(NodeRef::var("concept_id")).update_node(vec![
        ("source_count", count_value),
    ]))
    .returning(["updated"]);
```

---

## Typical Query Patterns

```rust
// Create a Recalls edge (base — no properties)
g().n(NodeRef::var("engram_id"))
    .add_e("Recalls", NodeRef::var("concept_id"), vec![])

// Create a Recalls edge with optional relationship embedding and strength
g().n(NodeRef::var("engram_id"))
    .add_e("Recalls", NodeRef::var("concept_id"), vec![
        ("relationship_embedding", edge_embedding),
        ("strength", 0.85),
    ])

// What concepts does this engram mention?
g().n(NodeRef::var("engram_id"))
    .out("Recalls")

// What engrams mention this concept?
g().n(NodeRef::var("concept_id"))
    .in("Recalls")

// Cognitive chain to identity
g().n(NodeRef::var("engram_id"))
    .out("Recalls")
    .out("Defines")

// Contradiction candidates: get engrams recalling same concept
g().n(NodeRef::var("concept_id"))
    .in("Recalls")

// Count engrams per concept
g().n(NodeRef::var("concept_id"))
    .in("Recalls")
    .count()

// Mitosis clustering: get engrams + their embeddings for a concept
g().n(NodeRef::var("concept_id"))
    .in("Recalls")
    .property("embedding")
```
