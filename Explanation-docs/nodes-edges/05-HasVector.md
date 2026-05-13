# E::HasVector — Engram ↔ EngramVector

## What It Is

Connects an `Engram` node to its `EngramVector` embedding. This is the bridge between the graph world (nodes, edges, traversals) and the vector world (embeddings, similarity search). Every engram has exactly one `HasVector` edge pointing to its embedding.

---

## Schema

    E::HasVector {
        From: Engram,
        To: EngramVector
    }

No properties — this is a simple structural link.

---

## Direction

    Engram ──HasVector──→ EngramVector

    From: Engram        (the memory)
    To:   EngramVector  (the memory's semantic embedding)

---

## When It's Created

| Trigger | Query | Timing |
|---|---|---|
| Engram ingestion | `CreateFullEngram` (atomic: creates engram + vector + edge in one query) | Immediately after engram creation |
| Manual linking | `ConnectEngramVector(engram_id, vector_id)` | Any time, but typically only at creation |

---

## How It's Used

| Use Case | Query Pattern | Direction |
|---|---|---|
| **Get engram's embedding** | `N<Engram>(id)::Out<HasVector>::FIRST` | Forward: engram → vector |
| **Search → get engrams** | `SearchV<EngramVector>(...)::In<HasVector>` | Backward: vector → engram |
| **Retrieve engram with vector** | `GetEngramWithVector(engram_id)` | Both directions in one query |

---

## Key Traversal: From Search to Engram

This is the single most important traversal in MNEMOS:

    SearchV<EngramVector>(Embed(query), 20)   // Find top-20 similar vectors
      ::In<HasVector>                          // Walk back to Engram nodes

Without this edge, vector search results are just vectors — you can't get back to the engram data (episode text, emotions, timestamps). The `HasVector` edge makes this bidirectional.

---

## Lifecycle

| Event | Effect on Edge |
|---|---|
| Engram created | Edge created (via `CreateFullEngram`) |
| Engram compressed (level 1) | Edge preserved — vector stays even when raw text is cleared |
| Engram promoted (level 2) | Edge preserved |
| Engram pruned (dropped) | Edge cascade-deleted (HelixDB drops all edges connected to a dropped node) |
| Vector dropped | Edge cascade-deleted |

**Warning:** When an engram is pruned, its vector must also be explicitly dropped to avoid orphan vectors in the search index. The `PruneEngram` query should include a `DROP` on the vector too, or a cleanup job must periodically remove vectors with no incoming `HasVector` edges.

---

## Relationships

| Entity | Relationship |
|---|---|
| `Engram` | Source node — owns the edge |
| `EngramVector` | Target node — is pointed to |
| `SearchV` | Uses the vector index built on `EngramVector`, then traverses `In<HasVector>` to get engrams |
| Mitosis Splitter | Reads vector data for clustering, returns via this edge |
