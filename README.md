# MNEMOS — Cognitive Memory Operating System

> AI memory system built on HelixDB 3.0.0 with native graph + vector search

---

## Start Here (Implementation Guides)

These are the authoritative references for building MNEMOS:

| # | Doc | What It Covers |
|---|-----|----------------|
| 1 | [Integration Guide](./helixdb-embedded-integration.md) | SDK setup, client init, storage backends, cache config |
| 2 | [Schema Definition](./helixdb-embedded-schema.md) | Nodes, edges, indexes in Rust DSL |
| 3 | [Query Reference](./helixdb-embedded-queries.md) | All `#[query]` functions with Rust DSL |
| 4 | [Retrieval Layer](./helixdb-embedded-retrieval.md) | CRR formula + Stimulation Layer |
| 5 | [Full Pipeline](./helixdb-embedded-pipeline.rs) | Complete reference implementation |

---

## Architecture & Design

| Doc | What It Covers |
|-----|----------------|
| [FutureMemoryAGI.md](./FutureMemoryAGI.md) | System overview, 5 algorithms, build path |
| [Cognitive-Tool-Protocol.md](./Cognitive-Tool-Protocol.md) | CTP protocol design (thinking stream injection) |
| [Two-Options-Query-Achetecture-Layer.md](./Two-Options-Query-Achetecture-Layer.md) | Query layer comparison (tool-call vs CTP) |

---

## Algorithm Deep Dives

### Core Algorithms

| Doc | Algorithm |
|-----|-----------|
| [01-Emotional-Tagger.md](./Explanation-docs/01-Emotional-Tagger.md) | Sentiment/emotion scoring at ingestion |
| [02-Importance-Scorer.md](./Explanation-docs/02-Importance-Scorer.md) | Memory importance classification |
| [03-Concept-Extractor.md](./Explanation-docs/03-Concept-Extractor.md) | NLP concept extraction |
| [04-Contradiction-Detector.md](./Explanation-docs/04-Contradiction-Detector.md) | Find conflicting memories |
| [05-Memory-Mitosis-Splitter.md](./Explanation-docs/05-Memory-Mitosis-Splitter.md) | Concept splitting via clustering |
| [06-Resonance-Scorer.md](./Explanation-docs/06-Resonance-Scorer.md) | CRR — Cognitive Resonance Retrieval |
| [07-Stimulation-Layer.md](./Explanation-docs/07-Stimulation-Layer.md) | Spreading activation dynamics |
| [08-Learnable-Edge-Weights.md](./Explanation-docs/08-Learnable-Edge-Weights.md) | Adaptive association strengths |

### Node & Edge Definitions

| Doc | Entity |
|-----|--------|
| [01-Engram.md](./Explanation-docs/nodes-edges/01-Engram.md) | Core memory unit (with embedding) |
| [03-Concept.md](./Explanation-docs/nodes-edges/03-Concept.md) | Knowledge concept (optional centroid) |
| [04-Identity.md](./Explanation-docs/nodes-edges/04-Identity.md) | Personality trait |
| [06-Recalls.md](./Explanation-docs/nodes-edges/06-Recalls.md) | Engram → Concept (optional edge embedding) |
| [07-AbstractsTo.md](./Explanation-docs/nodes-edges/07-AbstractsTo.md) | Engram → Concept abstraction |
| [08-Contradicts.md](./Explanation-docs/nodes-edges/08-Contradicts.md) | Engram ↔ Engram contradiction |
| [09-Reinforces.md](./Explanation-docs/nodes-edges/09-Reinforces.md) | Engram ↔ Engram reinforcement |
| [10-TemporalSequence.md](./Explanation-docs/nodes-edges/10-TemporalSequence.md) | Chronological ordering |
| [11-SpawnedFrom.md](./Explanation-docs/nodes-edges/11-SpawnedFrom.md) | Parent → Child (mitosis) |
| [12-Defines.md](./Explanation-docs/nodes-edges/12-Defines.md) | Concept → Identity |

### Query Patterns by Category

| Doc | Category |
|-----|----------|
| [01-Create-Queries.md](./Explanation-docs/queries/01-Create-Queries.md) | Write operations |
| [02-Read-Queries.md](./Explanation-docs/queries/02-Read-Queries.md) | Read operations |
| [03-Vector-Search.md](./Explanation-docs/queries/03-Vector-Search.md) | Semantic search |
| [04-Consolidation.md](./Explanation-docs/queries/04-Consolidation.md) | Decay/compress/promote |
| [05-Identity.md](./Explanation-docs/queries/05-Identity.md) | Identity operations |
| [06-Contradiction.md](./Explanation-docs/queries/06-Contradiction.md) | Contradiction detection |
| [07-Temporal.md](./Explanation-docs/queries/07-Temporal.md) | Timeline queries |
| [08-Stats.md](./Explanation-docs/queries/08-Stats.md) | Aggregations |

### Query Layer (CTP)

| Doc | Protocol |
|-----|----------|
| [01-Tool-Call-Protocol.md](./Explanation-docs/query-layer/01-Tool-Call-Protocol.md) | Function-calling tools |
| [02-Native-Token-Protocol.md](./Explanation-docs/query-layer/02-Native-Token-Protocol.md) | Thinking stream injection |

---

## Reference

| Doc | Purpose |
|-----|---------|
| [helixdb-vector-direct-vs-separate-node.md](./helixdb-vector-direct-vs-separate-node.md) | Why direct-on-node vector storage was chosen |

---

## Key Design Decisions

1. **Direct-on-node vector storage** — Embeddings stored on `Engram.embedding`, not separate `EngramVector` nodes
2. **Rust DSL** — All queries are `#[query]` functions, no HelixQL
3. **Embedded execution** — Database runs in-process, no HTTP server
4. **Optional enhancements** — Concept centroid embedding, edge embeddings, dynamic weights

---

## Quick Start

```bash
# 1. Add dependency
cargo add helix-db@3.0.0 --features embedded

# 2. Open embedded database
let client = Client::open(HelixDbSource::Disk {
    root: "/data/helix".into(),
    database: "mnemos".to_string(),
}).await?;

// 3. Create vector index
let index_request = create_vector_index();
client.query(index_request).send().await?;

// 4. Ingest a memory
let engram_id = pipeline.ingest_memory("User talked about...", "episodic").await?;

// 5. Retrieve relevant memories
let results = pipeline.retrieve_memories("query text", 10).await?;
```

See [Integration Guide](./helixdb-embedded-integration.md) for full setup instructions.
