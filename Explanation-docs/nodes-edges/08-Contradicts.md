# E::Contradicts — Engram → Engram

## What It Is

Links two engrams that **contradict** each other about the same concept. When the AI has experienced conflicting information (e.g., "remote work boosted productivity" vs. "remote work destroyed collaboration"), these two engrams get a `Contradicts` edge between them.

---

## Schema

```rust
g().n(NodeRef::var("engram_a_id")).add_e("Contradicts", NodeRef::var("engram_b_id"), vec![])
```

---

## Direction

```
Engram A ──Contradicts──→ Engram B
```

The direction is arbitrary — contradiction is symmetric. Both engrams also get `contradiction_flag = true` set on their nodes.

---

## When It's Created

| Trigger | Query | Timing |
|---|---|---|
| Contradiction Detector finds opposing emotional charges on same concept | `ConnectContradiction(a_id, b_id)` | At ingestion (step 4) and during consolidation |

---

## Downstream Effects

| Effect | How |
|---|---|
| CRR penalty | Both engrams get 0.5× multiplier in resonance formula |
| Retrieval suppression | `exclude_contradictions: true` in `SearchEngramsFull` filters them out entirely |
| Consolidation attention | Concepts with many contradiction edges get flagged for resolution |
| Identity protection | If a contradiction involves an identity-defining concept, the identity trait resists change rather than flipping |

---

## Resolution

During consolidation, contradictions can be resolved by:
1. **Splitting the concept** — the contradiction was really about two different sub-topics
2. **Adjusting confidence** — the concept's confidence drops to reflect uncertainty
3. **Temporal resolution** — newer evidence overrides older if the older engram is decaying

---

## Typical Query Patterns

```rust
// Create a Contradicts edge
g().n(NodeRef::var("engram_a_id"))
    .add_e("Contradicts", NodeRef::var("engram_b_id"), vec![])

// Get engrams that contradict this one
g().n(NodeRef::var("engram_id"))
    .out("Contradicts")

// Get engrams that this one contradicts (incoming)
g().n(NodeRef::var("engram_id"))
    .in("Contradicts")

// Find all contradicted engrams
g().n(NodeRef::var("engram_id"))
    .filter_node(PropertyFilter::property("contradiction_flag").eq(true))
    .out("Contradicts")
```
