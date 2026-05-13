# Vector Search Queries — Finding Relevant Memories

These queries power the retrieval half of MNEMOS. They use HelixDB's `SearchV` (cosine similarity search on vector index) to find engrams semantically similar to a query.

---

## 1. SearchEngramsByVector

### What It Does
Pure vector similarity search. Takes a pre-computed `[F64]` array and returns the top-k most similar `EngramVector` nodes by cosine distance. **Returns vectors only** — you must traverse `In<HasVector>` to get engram data (or use `SearchEngramsFull` instead).

### Parameters
| Param | Type | Source |
|---|---|---|
| `vector` | [F64] | External embedding model (1536 floats) |
| `limit` | I64 | How many results to return |

### Used By
- Custom embedding pipelines (you embed the query in your own code, pass the raw vector here)
- When you want raw vectors back without engram data
- Rarely used directly — `SearchEngramsFull` is preferred

---

## 2. SearchEngramsByText

### What It Does
Text-to-vector search. Takes a natural language query, embeds it server-side via `Embed()`, and returns the top-k most similar `EngramVector` nodes. Like `SearchEngramsByVector` but accepts text instead of a raw vector.

### Parameters
| Param | Type | Source |
|---|---|---|
| `query` | String | User's natural language query |
| `limit` | I64 | How many results to return |

### Returns
Top-k EngramVector nodes (vectors only, not engram data).

### Used By
- Quick recall without post-filtering
- Testing embedding quality
- When you need raw vectors for downstream processing

---

## 3. SearchEngramsFull (PRIMARY RETRIEVAL QUERY)

### What It Does
The **main retrieval query**. Does everything server-side in one operation:
1. Embeds the query text → vector
2. `SearchV` — finds top-k similar vectors
3. `::In<HasVector>` — traverses back to engram nodes
4. `::WHERE` — filters by importance threshold and contradiction flag
5. Returns engram data with selected properties

### Parameters
| Param | Type | Source | Purpose |
|---|---|---|---|
| `query` | String | User's natural language query | Embedded via `Embed()` server-side |
| `limit` | I64 | Application (default 20) | Top-k candidates from SearchV |
| `min_importance` | F32 | Application (default 0.0) | Filters out trivial engrams |
| `exclude_contradictions` | Boolean | Application (default true) | If true, skips contradicted engrams entirely |

### Returns
Filtered engram nodes with properties:
    [
        { ID, episode_raw, emotional_charge, importance_score, engram_type, timestamp },
        ...
    ]

### Used By
- `Pipeline.retrieve_memories()` — the primary retrieval pipeline
- Every user query that requires memory lookup

### Why This Isn't a Keyword Search
The `Embed(query)` step converts the query to the same 1536-dim vector space as stored engrams. The `SearchV` then finds vectors with highest cosine similarity. This means:
- "African tech infrastructure" matches "Uganda ICT Hub in Kampala" (semantic, not keyword)
- "How do I feel about remote work?" matches "Remote work boosted our productivity" (conceptual overlap)
- Typos and synonyms are handled by the embedding model

### Post-Processing (in Rust)
After this query returns candidates, the Resonance Scorer computes the full CRR formula:
```
resonance = semantic_sim × recency × emotion_boost × identity_alignment × contradiction_penalty
```
The server-side query handles Factors 1 (semantic similarity) and part of 5 (contradiction filter). The remaining factors are computed in Rust.

---

## SearchV Under the Hood

    SearchV<EngramVector>(Embed(query), limit)

    1. Embed(query) → sends text to OpenAI/Gemini → returns [F64; 1536]
    2. HelixDB ANN index → finds top-k nearest neighbors by cosine distance
    3. Returns vector nodes with similarity scores

The ANN (Approximate Nearest Neighbor) index is maintained by HelixDB automatically. You don't need to configure anything beyond setting up the embedding model.

---

## Model Selection for Embedding

Default model: `text-embedding-ada-002` (OpenAI, 1536 dims).

Override per-query with `#[model]` macro:

    #[model("gemini:gemini-embedding-001:RETRIEVAL_QUERY")]
    QUERY SearchEngramsFull(query: String, limit: I64, ...) =>
        ...

This uses Gemini's task-optimized embedding (`RETRIEVAL_QUERY` hint for query-side embedding vs `RETRIEVAL_DOCUMENT` for storage-side embedding).
