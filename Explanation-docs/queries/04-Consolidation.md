# Consolidation Queries — Ebbinghaus Decay & Memory Lifecycle

These queries are called by the **consolidation loop** (`Pipeline.consolidate()`), which runs as a background process between user sessions — the AI's "sleep" cycle. They govern the full lifecycle of engrams: decay, compression, pruning, promotion, mitosis, and source count maintenance.

---

## 1. GetEngramsForDecayCheck

### What It Does
Returns all engrams of a given type that are still at `compression_level = 0` (raw, uncompressed). The application code then computes the Ebbinghaus retention formula in Rust and decides whether to compress, prune, or promote each engram.

### Parameters
| Param | Type | Purpose |
|---|---|---|
| `engram_type` | String | Typically "episodic" (episodic memories decay, semantic/crystallized don't) |

### Returns
    [{ ID, decay_rate, activation_count, timestamp, importance_score }, ...]

### Retention Formula (computed in Rust, NOT in HelixQL)
    days_elapsed = (now - timestamp) / 86400.0
    retention = exp(-decay_rate * days_elapsed / sqrt(activation_count + 1))

### Used By
- `Pipeline.consolidate()` — the main decay loop

---

## 2. CompressEngram

### What It Does
Compresses an engram: sets `compression_level = 1` and clears the `episode_raw` text (replaces with "COMPRESSED"). The engram node and edges are preserved, but the raw content is gone. This frees storage while keeping the structural graph connections.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | ID |

### Triggered When
    retention < 0.15 AND importance_score < 0.3

### Used By
- Consolidation loop — for engrams that are fading but still have structural value

---

## 3. PromoteToSemantic

### What It Does
Promotes an engram to permanent semantic memory: sets `engram_type = "semantic"` and `compression_level = 2` (crystallized). The engram's raw text and all edges are preserved. It will no longer be scanned for decay in future consolidation cycles.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | ID |

### Triggered When
    retention > 0.8 AND activation_count > 20

Meaning: the engram has been recalled many times and is resisting decay — it's become long-term knowledge.

### Used By
- Consolidation loop — for engrams that have proven themselves durable

---

## 4. PruneEngram

### What It Does
**Deletes** an engram node from the graph. HelixDB cascade-deletes all edges connected to the engram (`HasVector`, `Recalls`, `AbstractsTo`, `Contradicts`, `Reinforces`, `TemporalSequence`, `SpawnedFrom`).

### Parameters
| Param | Type |
|---|---|
| `engram_id` | ID |

### Triggered When
    retention < 0.05 AND importance_score < 0.1

Meaning: the engram is mostly forgotten AND was never important.

### Important
The engram's `EngramVector` is NOT automatically dropped — the cascade only deletes edges, not the vector node. A cleanup step must drop orphan vectors (vectors with no incoming `HasVector` edges).

### Used By
- Consolidation loop — for engrams that have been forgotten

---

## 5. ActivateEngram

### What It Does
Increments the `activation_count` on an engram by 1. Called every time an engram is recalled during retrieval. Higher `activation_count` slows future decay — frequently accessed memories resist forgetting.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | ID |

### Used By
- `Pipeline.retrieve_memories()` — after returning search results, bumps activation_count on top-ranked engrams
- Not called during consolidation — only during retrieval

### Effect on Decay
    retention = exp(-decay_rate * days / sqrt(activation_count + 1))

    activation_count = 0  → sqrt(1) = 1.0   → normal decay
    activation_count = 3  → sqrt(4) = 2.0   → decay is halved
    activation_count = 15 → sqrt(16) = 4.0  → decay is quartered
    activation_count = 99 → sqrt(100) = 10.0 → decay is 10x slower

This is the **spacing effect** — the more you recall something, the slower you forget it.

---

## 6. CheckMitosisCandidates

### What It Does
Returns all concepts whose `source_count` meets or exceeds a threshold. These are candidates for memory mitosis — the concept may be overloaded and ready to split into specialized children.

### Parameters
| Param | Type | Purpose |
|---|---|---|
| `min_engram_count` | U32 | Minimum source_count to trigger mitosis (e.g., 10) |

### Returns
    [{ ID, name, confidence, source_count }, ...]

### Used By
- `Pipeline.consolidate()` — mitosis check step

### What Happens Next (in Rust)
For each concept returned:
    1. GetEngramsRecallingConcept(concept_id) → engram IDs
    2. For each engram: GetEngramWithVector(engram_id) → embeddings
    3. Cluster embeddings (HDBSCAN / K-means)
    4. For each cluster:
       a. Generate child concept name (LLM or centroid-based)
       b. CreateConcept(name, confidence) → child_id
       c. ConnectSpawn(parent_id, child_id)
       d. Reassign engrams: drop old Recalls/AbstractsTo, create new ones to child
    5. UpdateConceptSourceCount(parent_id) — recalculate after reassignment

---

## 7. UpdateConceptSourceCount

### What It Does
Recalculates the `source_count` on a concept by counting the number of incoming `Recalls` edges. Keeps the `source_count` field in sync with actual graph structure. Should be called after any operation that changes `Recalls` edges (ingestion, mitosis reassignment, pruning).

### Parameters
| Param | Type |
|---|---|
| `concept_id` | ID |

### Used By
- `Pipeline.ingest_memory()` — after adding Recalls edges for a new engram
- Mitosis Splitter — after reassigning engrams to child concepts
- Any time Recalls edges change

---

## Consolidation Loop Execution Order

    Pipeline.consolidate():

      1. DECAY
         engrams = GetEngramsForDecayCheck("episodic")
         For each engram:
           retention = compute in Rust
           if retention < 0.05 && importance < 0.1:
             PruneEngram(id)
             Cleanup orphan vector
           elif retention < 0.15 && importance < 0.3:
             CompressEngram(id)
           elif retention > 0.8 && activation_count > 20:
             PromoteToSemantic(id)

      2. MITOSIS
         concepts = CheckMitosisCandidates(min=10)
         For each concept:
           engram_embeddings = GetEngramsRecallingConcept(id) → vectors
           clusters = HDBSCAN(embeddings)
           For each cluster:
             CreateConcept(child_name, confidence)
             ConnectSpawn(parent_id, child_id)
             Reassign Recalls/AbstractsTo edges
             UpdateConceptSourceCount(child_id)
           UpdateConceptSourceCount(parent_id)

      3. CONTRADICTION SCAN
         For each concept:
           engrams = GetEngramsRecallingConcept(id)
           Group by emotional_charge sign
           Detect cross-group contradictions → ConnectContradiction

      4. IDENTITY CRYSTALLIZATION
         For each concept:
           if confidence > 0.7 && source_count > 20:
             Create or update Identity via weighted blending
