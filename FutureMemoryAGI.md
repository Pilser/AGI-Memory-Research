#                                 AGI Memory: The MNEMOS
---

# MNEMOS — Cognitive Memory Operating System

## What makes this different from everything that exists

Every current "AI memory" system is fundamentally RAG: embed text → store vector → retrieve similar chunks → inject into prompt. That's not memory. That's a search engine feeding a stateless model. MNEMOS is architecturally different in seven ways.

---

## The Engram — The Core Unit That Doesn't Exist Yet

Current systems store documents or chunks. MNEMOS stores **engrams** — cognitive memory units modeled loosely on how neuroscience describes biological memory traces. Here's the full HelixQL schema:

```hql
N::Engram {
  episode_raw:       String,
  semantic_vector:   Vector(1536),
  emotional_charge:  F32,           // -1.0 to +1.0
  importance_score:  F32,           // 0.0 to 1.0
  decay_rate:        F32,           // Ebbinghaus parameter
  activation_count:  U32,           // times recalled
  timestamp:         U64,
  engram_type:       String,        // "episodic"|"semantic"|"procedural"
  compression_level: U8,            // 0=raw, 1=compressed, 2=crystallized
  contradiction_flag: Bool
}

N::Concept {
  name:              String,
  semantic_vector:   Vector(1536),
  confidence:        F32,
  formation_date:    U64,
  source_count:      U32
}

N::Identity {
  trait:             String,
  value:             F32,
  stability:         F32,           // resistance to change
  last_updated:      U64
}

E::recalls {}              // Engram → Concept
E::contradicts {}          // Engram → Engram
E::reinforces {}           // Engram → Engram
E::abstracts_to {}         // Engram → Concept
E::defines {}              // Concept → Identity
E::temporal_sequence {}    // Engram → Engram (time order)
E::spawned_from {}         // Engram → Engram (after mitosis)
```

---

## The Five Algorithms That Don't Exist

**1. Cognitive Resonance Retrieval (CRR)** — not similarity search:

```
resonance(query, engram) =
  semantic_sim(q.vector, e.vector)
  × recency_weight(e.timestamp, now)
  × (1 + |e.emotional_charge|)
  × identity_alignment(e, identity_graph)
  × (1 − e.contradiction_flag × 0.5)
```

This is why the AI surfaces the memory about Uganda ICT Hub when you ask about infrastructure, even if the semantic distance isn't minimal — because it's identity-aligned. That's not RAG. That's cognition.

**2. Memory Mitosis** — concept emergence from complexity:

When an engram's out-degree (number of `abstracts_to` edges) crosses a threshold, it **divides** into two child nodes. Each inherits the parent's `semantic_vector` but gets fine-tuned independently via the diverging experiences that activate each child. This is how "I understand databases" fractures into "I understand write-ahead logs" and "I understand query planners" — two specialized concepts born from one root engram.

**3. Adaptive Ebbinghaus Decay** — applied to a graph, not a spreadsheet:

```python
retention = exp(-(decay_rate × Δt) / sqrt(activation_count + 1))
# If retention < 0.15 and importance_score < 0.3:
#   → compress engram (summary only, drop raw episode)
# If retention < 0.05 and importance_score < 0.1:
#   → prune (delete node, preserve concept edges)
# If retention > 0.8 and activation_count > 20:
#   → promote to semantic memory (compression_level = 2)
```

The graph runs this as a background HelixQL query every consolidation cycle.

**4. Reflective Consolidation Loop** — the sleep cycle:

Between sessions, a background process runs that no current AI system has:
- Traverses all recent episodic engrams
- Extracts semantic concepts via ML extraction
- Detects `contradicts` edge clusters → runs contradiction resolution
- Triggers mitosis on overloaded nodes
- Updates `Identity` nodes from the new semantic graph
- Rebuilds the resonance index

The AI literally "sleeps" between conversations and wakes up cognitively different — more organized, less contradictory, more itself.

**5. Identity Crystallization** — stable personality over years:

The `Identity` subgraph changes slowly, with `stability` values resisting updates. If new experiences contradict a highly stable identity trait, they get flagged for contradiction resolution rather than overwriting. The AI doesn't casually unlearn who it is.

---

## Why HelixDB Specifically

Vector databases handle semantic similarity, while graph databases handle relationships — most AI-driven applications need both, but until now developers had to build on top of two different systems with no native way to perform queries that span both.

MNEMOS needs to do, in a single atomic operation: "find engrams semantically close to this query, that have strong identity alignment, that were formed after timestamp X, that don't have contradiction flags, traversing the concept graph to include connected beliefs." That query is impossible in any separated system. In HelixDB it's one compiled HelixQL query.

HelixDB combines graph traversal, vector search, and HelixQL — a strongly typed compiled query language — in one platform, with private-by-default deployments and authenticated query access. That last part matters for identity persistence: the AI's cognitive state is private by default, not exposed as a public endpoint.

---

## What This Architecture Enables That Doesn't Exist Today

| Capability | Current AI | MNEMOS |
|---|---|---|
| Memory across sessions | Stateless / external DB hack | Native cognitive graph |
| Why something is remembered | Unknown | Traceable resonance score |
| Belief evolution over time | None | Temporal edge history |
| Contradiction handling | Ignored | Active resolution engine |
| Identity persistence | Prompt injection | Crystallized identity subgraph |
| Forgetting intelligently | Never or all-at-once | Ebbinghaus per-engram decay |
| Concept abstraction | None | Memory mitosis |
| Memory consolidation | None | Reflective sleep cycle |
| Context window | Hard token limit | Dynamic resonance activation |

---

## The Build Path

This is genuinely buildable today, in order:

1. **Phase 1** — Deploy HelixDB locally, implement the Engram schema, write basic CRR query in HelixQL. Store and retrieve engrams with resonance scoring. This alone beats every current memory system.

2. **Phase 2** — Build the Importance Scorer (small classifier trained on your own data) and Emotional Tagger. Wire them into the ingestion pipeline.

3. **Phase 3** — Implement the Consolidation Loop as a background Python/Rust service. Start with just decay + compression. Add contradiction detection.

4. **Phase 4** — Build Memory Mitosis. This is the hardest part — requires tracking concept complexity and splitting node embeddings correctly.

5. **Phase 5** — Identity Graph. Let the crystallization accumulate over real usage. After 3-6 months, the identity layer becomes genuinely meaningful.

This is not a research paper. This is architecture. The tools — HelixDB, HelixQL, Python ML, Rust services — all exist right now. What's never existed is putting them together this way.
