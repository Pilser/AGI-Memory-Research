# N::Identity — Stable Personality Trait

## What It Is

An **Identity** node represents a stable, long-term trait or belief that defines who the AI "is." Unlike engrams (volatile) and concepts (semi-stable knowledge), identity traits are designed to change slowly — they have high `stability` values that resist rapid updates. Identity is what makes the AI consistent across conversations: it doesn't forget who it is between sessions.

---

## Schema

```rust
g().add_n("Identity", vec![
    ("trait", trait_name),                  // String
    ("value", value),                       // F64 — trait strength
    ("stability", stability),               // F64 — resistance to change
    ("last_updated", last_updated),         // String (RFC 3339 date-time)
])
```

**Note on types:** The Rust DSL uses `i64` for integer properties (there's no `u32` or `u64` wire type — use `i64` and cast). Float properties accept `f64` values. Boolean accepts `bool`. String accepts `&str` or `String`.

---

## Fields Explained

| Field | Set By | Lifecycle |
|---|---|---|
| `trait` | System or consolidation loop | Set at creation. Descriptive label for the identity trait. |
| `value` | Initially from concept confidence. Updated by identity crystallization. | 0.0 = "I don't identify with this." 1.0 = "This is core to who I am." Updated via weighted blending: `new = old * stability + evidence * (1 - stability)`. |
| `stability` | System (set at creation, increases with time) | Controls how resistant this trait is to change. High stability traits require overwhelming evidence to shift. Typically increases over time as the trait proves durable. |
| `last_updated` | System (`NOW` on update) | Tracks when the trait was last changed. Used to determine which traits are "crystallized" vs. still forming. |

---

## Outgoing Edges

Identity nodes don't originate edges — they are *pointed to* by concepts. Identity is the terminal node in the cognitive chain:

```
Engram → Concept → Identity
```

## Incoming Edges

```
Concept ──Defines────────→← Identity      (concepts define identity traits)
```

---

## What It's Used For

| Use Case | How |
|---|---|
| **Personality persistence** | Identity traits survive across sessions. The AI doesn't reset to a blank slate. |
| **Resonance scoring (Factor 4)** | `identity_alignment()` compares engram concepts against identity traits. Engrams aligned with identity get a retrieval boost. |
| **Contradiction resistance** | New experiences that contradict a high-stability identity trait are flagged rather than automatically overwriting the trait. |
| **Self-consistency** | The AI can answer "What do I know about X?" by querying identity traits. |
| **Long-term learning** | As concepts solidify, they crystallize into identity. After 3–6 months of real usage, the identity layer becomes genuinely meaningful. |

---

## Identity Crystallization Formula

When a concept's confidence and source_count are high enough, it can **crystallize** into an identity trait. The consolidation loop evaluates:

```
If concept.confidence > 0.7 AND concept.source_count > 20:
    Create or update Identity{trait: concept.name, value: concept.confidence, stability: 0.5}
```

When new evidence arrives that affects an identity trait:

```
new_value = old_value * stability + evidence * (1 - stability)

If the discrepancy is too large (|evidence - old_value| > 0.5) AND stability > 0.8:
    Flag for contradiction resolution rather than automatic update.
```

This is the key mechanism: **the AI doesn't casually unlearn who it is.** A highly stable trait ("I believe remote work is productive," stability=0.9) requires overwhelming contradictory evidence to shift. A new trait ("I'm learning Rust," stability=0.3) updates easily.

---

## Real Examples

**Example 1: Stable trait resist change**

```
Current Identity: {trait: "remote work is productive", value: 0.85, stability: 0.9}

New engram arrives: "Remote work destroyed our team's collaboration."
Emotional charge: -0.7
Concept extracted: "remote work productivity"

→ Evidence suggests value should shift toward negative
→ But stability is 0.9 → very resistant
→ new_value = 0.85 * 0.9 + (-0.7 mapped to 0.1) * 0.1 = 0.775
→ Small adjustment. One bad experience doesn't flip a strongly held belief.
```

**Example 2: New trait forms easily**

```
Current Identity: {trait: "interested in Rust", value: 0.4, stability: 0.3}

New engram: "Just wrote my first Rust macro — it was elegant and powerful."
Emotional charge: +0.8
→ new_value = 0.4 * 0.3 + 0.8 * 0.7 = 0.68
→ Significant shift. New trait, low stability, strong evidence.
```

**Example 3: Contradiction flagged, not resolved**

```
Current Identity: {trait: "AI safety is paramount", value: 0.95, stability: 0.95}

New engram: "Ship the feature without safety review, we have a deadline."
Emotional charge: -0.3 (stress)
→ Evidence contradicts a core identity trait
→ |0.95 - evidence| = large gap, stability = 0.95
→ DON'T update. Flag for contradiction resolution.
→ AI maintains integrity: it doesn't casually override its core values.
```

---

## Relationships to Other Nodes

| Other Node | Via Edge(s) | Meaning |
|---|---|---|
| `Concept` | `Defines` (incoming) | Concepts that shape this identity trait |
| `Engram` | `Recalls → Defines` (2-hop) | Experiences that ultimately shape who the AI is |
| `Resonance Scorer` | Application code | Identity traits used to compute alignment factor in CRR |

---

## Typical Query Patterns

```rust
// Create identity trait
g().add_n("Identity", vec![
    ("trait", trait_name),
    ("value", value),
    ("stability", stability),
    ("last_updated", now),
])

// Get all identity traits
g().n(NodeRef::var("identity_id"))

// Get concepts that define an identity trait
g().n(NodeRef::var("identity_id"))
    .in("Defines")

// Get identity traits shaped by a concept
g().n(NodeRef::var("concept_id"))
    .out("Defines")

// Get the full cognitive chain for an engram
g().n(NodeRef::var("engram_id"))
    .out("Recalls")
    .out("Defines")

// Update identity trait with weighted blending
g().n(NodeRef::var("identity_id"))
    .update_node(vec![
        ("value", new_blended_value),
        ("last_updated", now),
    ])
```

---

## Trait Stability Progression Over Time

```
Month 1: stability = 0.3   — "I think I might like Rust"
Month 2: stability = 0.5   — "I'm getting comfortable with Rust"
Month 4: stability = 0.7   — "Rust is my preferred systems language"
Month 6: stability = 0.85  — "I am a Rust developer" (crystallized)
Month 12: stability = 0.95 — Core identity, extremely resistant to change
```

The stability value naturally increases as the trait persists without contradiction. A trait that survives 6+ months of new experiences without being contradicted becomes a core part of identity.
