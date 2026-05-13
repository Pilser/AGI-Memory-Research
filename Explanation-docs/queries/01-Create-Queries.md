# Create Queries — Writing Data to DB

These queries are called at **ingestion time** and during **consolidation** to write nodes, vectors, and edges into the graph.

---

## 1. CreateEngram

### What It Does
Creates a bare `Engram` node with all fields manually specified. Used when you need full control over every field — but usually you want `CreateFullEngram` instead, which also creates the vector and edge atomically.

### Parameters
| Param | Type | Source |
|---|---|---|
| `episode_raw` | String | User/system input (raw memory text) |
| `emotional_charge` | F32 | Emotional Tagger (ML) |
| `importance_score` | F32 | Importance Scorer (ML) |
| `decay_rate` | F32 | System (default 0.05) |
| `activation_count` | U32 | System (0 at creation) |
| `engram_type` | String | System ("episodic", "semantic", "procedural") |
| `compression_level` | U8 | System (0 = raw) |
| `contradiction_flag` | Boolean | System (false at creation) |

### Returns
The created Engram node (with auto-assigned ID and `timestamp: NOW`).

### Used By
- Manual engram creation (testing, debugging)
- Pipeline: not directly — use `CreateFullEngram` instead

---

## 2. CreateConcept

### What It Does
Creates a `Concept` node. Called by the Concept Extractor for each concept extracted from an engram.

### Parameters
| Param | Type | Source |
|---|---|---|
| `name` | String | Concept Extractor (ML) |
| `confidence` | F32 | Concept Extractor (confidence in this concept) |

### Returns
The created Concept node (with auto-assigned `formation_date: NOW`, `source_count: 1`).

### Used By
- `Pipeline.ingest_memory()` — one call per extracted concept
- Mitosis Splitter — when creating child concepts

### Important
Before calling this, always check if the concept already exists via `GetConceptByName`. Calling this for an existing concept creates a duplicate.

---

## 3. CreateIdentity

### What It Does
Creates an `Identity` node — a stable personality trait.

### Parameters
| Param | Type | Source |
|---|---|---|
| `trait` | String | System / consolidation loop |
| `value` | F32 | Derived from concept confidence (0.0–1.0) |
| `stability` | F32 | System (0.5 for new traits, increases with time) |

### Returns
The created Identity node (with `last_updated: NOW`).

### Used By
- Identity crystallization (consolidation loop)
- Manual identity seeding

---

## 4. CreateEngramVectorText

### What It Does
Creates an `EngramVector` from raw text using HelixDB's built-in `Embed()` function. The embedding model (OpenAI/Gemini) converts the text to a 1536-dim vector server-side.

### Parameters
| Param | Type | Source |
|---|---|---|
| `content` | String | The engram's `episode_raw` text |

### Returns
The created EngramVector node.

### Used By
- Can be used standalone, but normally called indirectly via `CreateFullEngram`
- Testing embedding quality

### Requires
`OPENAI_API_KEY` or `GEMINI_API_KEY` environment variable set. Configured embedding model in `config.hx.json`.

---

## 5. CreateEngramVector

### What It Does
Creates an `EngramVector` from a pre-computed `[F64]` array. Use this when you have your own embedding pipeline (custom model) and want to pass the vector directly, bypassing HelixDB's `Embed()`.

### Parameters
| Param | Type | Source |
|---|---|---|
| `vector` | [F64] | Custom embedding model (1536 floats) |
| `content` | String | The engram's text (metadata only) |

### Returns
The created EngramVector node.

### Used By
- Custom embedding pipelines (e.g., local ONNX model, sentence-transformers)
- When you embed text in Rust and pass the array directly

---

## 6. ConnectEngramVector

### What It Does
Creates a `HasVector` edge connecting an engram to its embedding vector.

### Parameters
| Param | Type | Source |
|---|---|---|
| `engram_id` | ID | From `CreateEngram` result |
| `vector_id` | ID | From `CreateEngramVector` result |

### Used By
- Manual linking (rare — normally done atomically by `CreateFullEngram`)
- Reconnecting after migration

---

## 7. ConnectRecalls

### What It Does
Creates a `Recalls` edge: Engram → Concept. "This memory mentions/recalls this concept."

### Parameters
| Param | Type | Source |
|---|---|---|
| `engram_id` | ID | The engram linking to the concept |
| `concept_id` | ID | The concept being recalled |

### Used By
- `Pipeline.ingest_memory()` — one call per extracted concept
- Mitosis — reassigning engrams to child concepts

---

## 8. ConnectContradiction

### What It Does
Creates a `Contradicts` edge between two engrams that contradict each other.

### Parameters
| Param | Type | Source |
|---|---|---|
| `engram_a_id` | ID | First engram |
| `engram_b_id` | ID | Second engram (contradicts the first) |

### Used By
- Contradiction Detector (ML) — when opposite emotional charges found on same concept
- Consolidation loop — batch contradiction scanning

---

## 9. ConnectReinforcement

### What It Does
Creates a `Reinforces` edge between two engrams that support/confirm each other.

### Parameters
| Param | Type | Source |
|---|---|---|
| `engram_a_id` | ID | Reinforcing engram |
| `engram_b_id` | ID | Reinforced engram |

### Used By
- Consolidation loop — when multiple engrams about the same concept have the same emotional sign

---

## 10. ConnectAbstraction

### What It Does
Creates an `AbstractsTo` edge: Engram → Concept. "This memory feeds into/abstracts to this concept." Paired with every `Recalls` edge.

### Parameters
| Param | Type | Source |
|---|---|---|
| `engram_id` | ID | The engram |
| `concept_id` | ID | The abstracted concept |

### Used By
- `Pipeline.ingest_memory()` — paired with `ConnectRecalls`
- Mitosis — reassigning abstractions to child concepts

---

## 11. ConnectDefinition

### What It Does
Creates a `Defines` edge: Concept → Identity. "This knowledge defines this identity trait."

### Parameters
| Param | Type | Source |
|---|---|---|
| `concept_id` | ID | The concept crystallizing into identity |
| `identity_id` | ID | The identity trait being defined |

### Used By
- Identity crystallization (consolidation loop)
- When a concept's confidence + source_count crosses threshold

---

## 12. ConnectTemporal

### What It Does
Creates a `TemporalSequence` edge from an earlier engram to a later one, establishing chronological order.

### Parameters
| Param | Type | Source |
|---|---|---|
| `earlier_id` | ID | The engram that happened first |
| `later_id` | ID | The engram that happened after |

### Used By
- Ingestion — links new engram to the previous most-recent engram
- Consolidation — batch timeline ordering

---

## 13. ConnectSpawn

### What It Does
Creates a `SpawnedFrom` edge: Parent Concept → Child Concept. "This specialized concept was spawned from this general concept."

### Parameters
| Param | Type | Source |
|---|---|---|
| `parent_id` | ID | The parent concept (general) |
| `child_id` | ID | The child concept (specialized, from mitosis) |

### Used By
- Memory Mitosis Splitter — one call per child concept spawned

---

## 14. CreateFullEngram (PRIMARY INGESTION QUERY)

### What It Does
The **main ingestion query**. Creates an engram node, embeds its text into a vector, and connects them with a `HasVector` edge — all in one atomic operation.

### Parameters
| Param | Type | Source |
|---|---|---|
| `episode_raw` | String | User/system input |
| `emotional_charge` | F32 | Emotional Tagger |
| `importance_score` | F32 | Importance Scorer |
| `decay_rate` | F32 | System (0.05) |
| `engram_type` | String | System ("episodic") |
| `compression_level` | U8 | System (0) |
| `contradiction_flag` | Boolean | System (false) |

### Returns
The created Engram node (ID used for subsequent edge creation).

### Used By
- `Pipeline.ingest_memory()` — the primary ingestion pipeline
- Every new memory enters through this query

### Atomic Guarantee
If `Embed()` fails (e.g., API key missing), the engram is NOT created. If engram creation fails, the vector is NOT created. All-or-nothing.

---

## Ingestion Call Order

```
Pipeline.ingest_memory(episode_raw):
  1. EmotionalTagger.tag(episode_raw)           → emotional_charge
  2. ImportanceScorer.score(episode_raw)        → importance_score
  3. CreateFullEngram(episode_raw, ...)          → engram_id
  4. ConceptExtractor.extract(episode_raw)       → [concepts...]
  5. For each concept:
       existing = GetConceptByName(concept.name)
       if !existing: CreateConcept(name, confidence) → concept_id
       ConnectRecalls(engram_id, concept_id)
       ConnectAbstraction(engram_id, concept_id)
       UpdateConceptSourceCount(concept_id)
  6. ContradictionDetector.check(engram_id)     → may call ConnectContradiction
```
