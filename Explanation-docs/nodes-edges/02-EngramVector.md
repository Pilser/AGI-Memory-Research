# V::EngramVector — Semantic Embedding

## What It Is

The **vector embedding** of an engram's raw text, stored as a separate vector entity in HelixDB (not as a field on the Engram node). This is how the system represents the **semantic meaning** of a memory in a high-dimensional space (1536 dimensions by default) for similarity search. It is the entry point for all retrieval — every memory query starts by searching EngramVectors.

---

## Schema

```hql
V::EngramVector {
    content: String   // A copy of the original text (for debugging/inspection)
}
```

The actual vector data is `[F64]` — an array of 1536 floating-point numbers — stored internally by HelixDB. The `content` field is metadata only.

---

## Why Vectors Are Separate Entities (Not Fields on Engram)

In the original MNEMOS design doc, the schema showed `semantic_vector: Vector(1536)` as a property on the Engram node. In real HelixDB, vectors are **first-class entities** (`V::` types) separate from nodes (`N::` types). This is because:

1. **Vector indexing** — HelixDB maintains ANN (Approximate Nearest Neighbor) indexes on `V::` types, enabling fast cosine similarity search via `SearchV`. Placing vectors as node properties would not benefit from this index.
2. **Connecting nodes to vectors** — The `HasVector` edge (`E::HasVector { From: Engram, To: EngramVector }`) links an engram to its embedding. This enables traversal from search results back to engram data in a single query.
3. **Multiple vectors per node** — An engram could theoretically have multiple embeddings (e.g., different models, different chunking strategies), each connected via a separate `HasVector` edge.

---

## Outgoing Edges

```
EngramVector ──HasVector (incoming)──← Engram
```

Vectors don't initiate edges — they are *pointed to* by Engram nodes. Search results traverse **backwards** from vector to node:
```
SearchV → In<HasVector> → Engram
```

---

## What It's Used For

| Use Case | Query | What Happens |
|---|---|---|
| **Text → vector on creation** | `AddV<EngramVector>(Embed(episode_raw), {content})` | HelixDB's built-in `Embed()` function sends the text to the configured embedding model (OpenAI/Gemini) and returns a 1536-dim vector |
| **Vector similarity search** | `SearchV<EngramVector>(Embed(query), limit)` | Finds the top-k vectors most semantically similar to the query |
| **Search → engram retrieval** | `SearchV<EngramVector>(...)::In<HasVector>` | Traverses from found vectors back to their Engram nodes |
| **Mitosis clustering** | Read embeddings of engrams linked to a concept via `GetEngramsRecallingConcept` → `GetEngramWithVector` | Embeddings are fed into clustering algorithms (DBSCAN/HDBSCAN) to split overloaded concepts |
| **Text-free vector creation** | `AddV<EngramVector>(vector_array, {content})` | If you have pre-computed embeddings (from a custom model), you can pass the raw `[F64]` array directly |

---

## Embedding Model

Configured in `config.hx.json` or `helix.toml`. Default: `text-embedding-ada-002` (OpenAI, 1536 dims). Overridable per-query with:

```hql
#[model("gemini:gemini-embedding-001:RETRIEVAL_DOCUMENT")]
QUERY CreateEngramVectorText(content: String) =>
    vector <- AddV<EngramVector>(Embed(content), {content: content})
    RETURN vector
```

---

## Lifecycle

```
1. Engram created → Embed(episode_raw) → vector stored via AddV
2. Engram compressed → vector REMAINS (structure preserved even when raw text dropped)
3. Engram pruned → DROP Engram cascades to DROP HasVector edges
                    → VECTOR IS ORPHANED and should be dropped too
4. Engram recalled → activation_count++ on Engram, vector unchanged
```

**Important:** When an engram is pruned, its vector must also be pruned to avoid orphan vectors polluting the search index. The `PruneEngram` query should cascade to the vector.

---

## Relationships to Other Entities

| Entity | Via | Meaning |
|---|---|---|
| `Engram` | `HasVector` edge (incoming) | "This vector represents that engram's semantic content" |
| `SearchV` | HelixDB vector index | Searches all EngramVector embeddings for nearest neighbors |
| Mitosis Splitter | Application code reads embeddings | Clusters embeddings to find concept sub-groups |

---

## Typical Query Patterns

```
-- Create vector from text (uses configured embedding model)
AddV<EngramVector>(Embed(text), {content: text})

-- Create vector from pre-computed array
AddV<EngramVector>(vector_array, {content: text})

-- Search by text (automatically embeds the query)
SearchV<EngramVector>(Embed(query), limit)

-- Search by raw vector
SearchV<EngramVector>(vector, limit)

-- Search → get engrams
SearchV<EngramVector>(Embed(query), limit)::In<HasVector>

-- Get engram → get its vector
N<Engram>(engram_id)::Out<HasVector>::FIRST
```
