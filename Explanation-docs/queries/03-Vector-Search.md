# Vector Search Queries — Finding Relevant Memories

These queries power the retrieval half of MNEMOS. They use HelixDB's vector search (cosine similarity on the `embedding` index) to find engrams semantically similar to a query.

> **Key simplification:** Vector search now operates directly on `Engram.embedding`. No `EngramVector` node type, no `HasVector` edge, no `::In<HasVector>` traversal. The search returns `Engram` nodes directly.

---

## How Vector Search Works Now

```
Old flow:  SearchV<EngramVector> → ::In<HasVector> → Engram
New flow:  SearchV<Engram> → Engram  (direct, one step)
```

The `vector_search_with()` DSL method takes a node label and property name. Since `embedding` is a property on the `Engram` node, the search returns engrams immediately — no traversal needed.

---

## 1. SearchEngramsByVector

### What It Does
Pure vector similarity search. Takes a pre-computed embedding vector and returns the top-k most similar `Engram` nodes by cosine distance. Returns engram data directly — no traversal needed.

### Parameters
| Param | Type | Source |
|---|---|---|
| `query_vector` | Vec<f32> | External embedding model (1536 dims, computed client-side) |
| `limit` | i64 | How many results to return |

### Returns
Top-k Engram nodes with similarity scores:
```json
{
    "results": [
        { "$id": 42, "$distance": 0.12, "episode_raw": "User discussed Uganda ICT..." },
        { "$id": 57, "$distance": 0.18, "episode_raw": "Kampala tech hub visit..." }
    ]
}
```

### Used By
- Custom embedding pipelines (you embed the query in your own code, pass the raw vector here)
- When you want raw engram data back with similarity scores
- `SearchEngramsFull` is preferred for most use cases (adds filtering)

### Query Implementation
```rust
use helix_db::dsl::prelude::*;

#[query]
fn search_engrams_by_vector(
    query_vector: Vec<f32>,
    limit: i64,
) -> ReadBatch {
    read_batch()
        .var_as("results", g()
            // Vector search returns Engram nodes DIRECTLY (no ::In<HasVector> needed)
            .vector_search_with("Engram", "embedding", query_vector, limit, None)
            .value_map(Some(vec!["$id", "$distance", "episode_raw"])))
        .returning(["results"])
}
```

### Typical Query Patterns
```rust
// Embed the query client-side, then search
let query_embedding = embed_text("African tech infrastructure").await?;
let request = search_engrams_by_vector(query_embedding, 20);
let response = client.query(request).send().await?;
let results: Vec<SearchResult> = serde_json::from_value(response["results"].clone())?;
```

---

## 2. SearchEngramsByText

### What It Does
Text-to-vector search convenience. Takes a natural language query, but since HelixDB embedded mode doesn't support server-side `Embed()`, the embedding must be computed client-side first. This function accepts the pre-computed embedding and returns the top-k most similar `Engram` nodes.

### Parameters
| Param | Type | Source |
|---|---|---|
| `query_embedding` | Vec<f32> | Embedding model (computed client-side from query text) |
| `limit` | i64 | How many results to return |

### Returns
Top-k Engram nodes with similarity scores.

### Used By
- Quick recall without post-filtering
- Testing embedding quality
- When you need raw vectors for downstream processing

### Query Implementation
```rust
#[query]
fn search_engrams_by_text(
    query_embedding: Vec<f32>,
    limit: i64,
) -> ReadBatch {
    read_batch()
        .var_as("results", g()
            // Vector search returns Engram nodes DIRECTLY
            .vector_search_with("Engram", "embedding", query_embedding, limit, None)
            .value_map(Some(vec!["$id", "$distance", "episode_raw"])))
        .returning(["results"])
}
```

### Typical Query Patterns
```rust
// Step 1: Embed the query text client-side
let query_text = "How do I feel about remote work?";
let query_embedding = embed_text(query_text).await?;

// Step 2: Search using the embedding
let request = search_engrams_by_text(query_embedding, 20);
let response = client.query(request).send().await?;
```

---

## 3. SearchEngramsFull (PRIMARY RETRIEVAL QUERY)

### What It Does
The **main retrieval query**. Does vector search + filtering in one operation:
1. Vector search on `Engram.embedding` — finds top-k similar engrams directly
2. `WHERE` — filters by importance threshold and contradiction flag
3. Returns engram data with selected properties

### Parameters
| Param | Type | Source | Purpose |
|---|---|---|---|
| `query_embedding` | Vec<f32> | Embedding model (client-side) | The query vector |
| `limit` | i64 | Application (default 20) | Top-k candidates from vector search |
| `min_importance` | f64 | Application (default 0.0) | Filters out trivial engrams |

### Returns
Filtered engram nodes with properties:
```json
{
    "candidates": [
        {
            "$id": 42,
            "episode_raw": "User discussed Uganda ICT infrastructure",
            "emotional_charge": 0.7,
            "importance_score": 0.8,
            "engram_type": "episodic",
            "timestamp": "2026-05-10T14:30:00Z"
        }
    ]
}
```

### Used By
- `Pipeline.retrieve_memories()` — the primary retrieval pipeline
- Every user query that requires memory lookup

### Why This Isn't a Keyword Search
The `query_embedding` converts the query to the same 1536-dim vector space as stored engrams. The vector search then finds embeddings with highest cosine similarity. This means:
- "African tech infrastructure" matches "Uganda ICT Hub in Kampala" (semantic, not keyword)
- "How do I feel about remote work?" matches "Remote work boosted our productivity" (conceptual overlap)
- Typos and synonyms are handled by the embedding model

### Post-Processing (in Rust)
After this query returns candidates, the Resonance Scorer computes the full CRR formula:
```
resonance = semantic_sim × recency × emotion_boost × identity_alignment × contradiction_penalty
```
The server-side query handles Factors 1 (semantic similarity) and part of 5 (contradiction filter). The remaining factors are computed in Rust.

### Query Implementation
```rust
#[query]
fn search_engrams_full(
    query_embedding: Vec<f32>,
    limit: i64,
    min_importance: f64,
) -> ReadBatch {
    read_batch()
        .var_as("candidates", g()
            // Step 1: Vector search returns Engram nodes DIRECTLY
            .vector_search_with("Engram", "embedding", query_embedding, limit, None)
            // Step 2: Filter by importance and contradiction flag (no traversal needed!)
            .where_(Predicate::gte("importance_score", min_importance))
            .where_(Predicate::eq("contradiction_flag", false))
            // Step 3: Project needed fields
            .value_map(Some(vec![
                "$id", "episode_raw", "emotional_charge",
                "importance_score", "engram_type", "timestamp",
            ])))
        .returning(["candidates"])
}
```

**Key simplification:** No `::In<HasVector>` traversal needed. Vector search returns `Engram` nodes directly.

---

## 4. Concept-Level Search (Optional Enhancement)

### What It Does
Searches for concepts by their centroid embedding rather than individual engrams. This enables finding broad knowledge areas before drilling into specific memories. Requires the optional `embedding` property on `Concept` nodes (computed via `UpdateConceptCentroid`).

### Parameters
| Param | Type | Source |
|---|---|---|
| `query_embedding` | Vec<f32> | Embedding model (client-side) |
| `limit` | i64 | How many concepts to return |

### Returns
Top-k Concept nodes by centroid similarity:
```json
{
    "concepts": [
        { "$id": 12, "$distance": 0.15, "name": "Uganda ICT", "confidence": 0.85 },
        { "$id": 28, "$distance": 0.22, "name": "remote work", "confidence": 0.72 }
    ]
}
```

### Used By
- Broad knowledge discovery — "What do I know about X?"
- Pre-filtering before engram-level search
- Concept graph navigation

### Query Implementation
```rust
#[query]
fn search_concepts_by_vector(
    query_embedding: Vec<f32>,
    limit: i64,
) -> ReadBatch {
    read_batch()
        .var_as("concepts", g()
            .vector_search_with("Concept", "embedding", query_embedding, limit, None)
            .value_map(Some(vec!["$id", "$distance", "name", "confidence"])))
        .returning(["concepts"])
}
```

### Typical Query Patterns
```rust
// Two-level search: first find relevant concepts, then drill into engrams
let query_embedding = embed_text("African tech infrastructure").await?;

// Level 1: Find relevant concepts
let concept_request = search_concepts_by_vector(query_embedding.clone(), 5);
let concept_response = client.query(concept_request).send().await?;

// Level 2: For each concept, get its engrams
for concept in &concept_response["concepts"] {
    let engram_request = get_engrams_recalling_concept(concept["$id"].as_u64());
    let engram_response = client.query(engram_request).send().await?;
    // Process engrams...
}
```

### Computing and Storing Concept Centroids
```rust
#[query]
fn update_concept_centroid(
    concept_id: u64,
    centroid_embedding: Vec<f32>,
) -> WriteBatch {
    write_batch()
        .var_as("updated", g()
            .n(NodeRef::id(concept_id))
            .set_property("embedding", centroid_embedding))
        .returning(["updated"])
}

// Client-side: compute centroid from assigned engrams
async fn compute_and_update_centroid(
    &self,
    concept_id: u64,
) -> Result<(), Box<dyn std::error::Error>> {
    // 1. Get all engrams recalling this concept
    let engrams_request = get_engrams_recalling_concept(concept_id);
    let engrams_response = self.client.query(engrams_request).send().await?;
    let engrams: Vec<EngramData> = serde_json::from_value(
        engrams_response["engrams"].clone()
    )?;

    // 2. Compute centroid (average of all engram embeddings)
    let centroid = compute_centroid(&engrams);

    // 3. Store centroid on the concept
    let update_request = update_concept_centroid(concept_id, centroid);
    self.client.query(update_request).send().await?;

    Ok(())
}

fn compute_centroid(engrams: &[EngramData]) -> Vec<f32> {
    let dim = 1536;
    let mut centroid = vec![0.0f32; dim];
    for engram in engrams {
        for (i, val) in engram.embedding.iter().enumerate() {
            centroid[i] += val;
        }
    }
    let n = engrams.len() as f32;
    for val in &mut centroid {
        *val /= n;
    }
    centroid
}
```

---

## Vector Search Under the Hood

```
vector_search_with("Engram", "embedding", query_vector, limit, None)

1. query_vector → the embedding computed client-side from query text
2. HelixDB ANN index on Engram.embedding → finds top-k nearest neighbors by cosine distance
3. Returns Engram nodes with $distance scores (no traversal needed!)
```

The ANN (Approximate Nearest Neighbor) index is maintained by HelixDB automatically on the `embedding` property of `Engram` nodes.

---

## Model Selection for Embedding

Default model: `text-embedding-ada-002` (OpenAI, 1536 dims).

Since HelixDB embedded mode doesn't support server-side `Embed()`, embeddings are computed client-side. The model choice is yours — just ensure the same model is used for both storage and query embedding.
