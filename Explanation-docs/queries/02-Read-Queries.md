# Read Queries — Reading Data from DB

These queries retrieve engrams, concepts, identity traits, and their relationships. Called at retrieval time, during consolidation, and for inspection.

---

## 1. GetEngram

### What It Does
Returns a single engram by its ID.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | ID |

### Used By
- Any pipeline step that needs to inspect a specific engram
- Debugging and testing

---

## 2. GetEngramWithVector

### What It Does
Returns an engram AND its embedding vector in one query. Uses `::Out<HasVector>` traversal to follow the edge from engram to its vector.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | ID |

### Returns
    {
        engram: { ID, episode_raw, emotional_charge, importance_score, ... },
        vector: { ID, content, ...embedding data... }
    }

### Used By
- Mitosis Splitter — needs both engram data and its embedding for clustering
- Resonance Scorer — needs embedding for identity alignment computation
- Any code that needs both the text and the vector

---

## 3. GetEngramsByType

### What It Does
Returns all engrams of a given type (e.g., all "episodic", all "semantic", all "procedural").

### Parameters
| Param | Type |
|---|---|
| `engram_type` | String ("episodic" / "semantic" / "procedural") |

### Used By
- Consolidation loop — processes episodic engrams for decay
- Stats and inspection

---

## 4. GetConceptByName

### What It Does
Looks up a concept by its `name` field. Requires an INDEX on `Concept.name`. Returns the concept if found, otherwise empty. This is the deduplication check — before creating a new concept, check if one with the same name already exists.

### Parameters
| Param | Type |
|---|---|
| `name` | String |

### Used By
- `Pipeline.ingest_memory()` — deduplication check before `CreateConcept`
- Concept Extractor — finding existing concepts to reuse

### Schema Requirement
    N::Concept {
        INDEX name: String,    // REQUIRED for this query
        ...
    }

---

## 5. GetRecalledConcepts

### What It Does
Returns all concepts that an engram recalls. Traverses `::Out<Recalls>` from the engram.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | ID |

### Used By
- Contradiction Detector — finds what concepts an engram relates to
- Identity alignment — traces engram → concepts chain
- Debugging — "what does this memory know about?"

---

## 6. GetEngramsRecallingConcept

### What It Does
Returns all engrams that recall a given concept. Traverses `::In<Recalls>` from the concept. The inverse of `GetRecalledConcepts`.

### Parameters
| Param | Type |
|---|---|
| `concept_id` | ID |

### Used By
- Mitosis Splitter — gets all engram embeddings linked to a concept for clustering
- Contradiction Detector — gets all engrams about a concept to check for opposing emotions
- `UpdateConceptSourceCount` — counts how many engrams feed a concept

---

## 7. GetContradictions

### What It Does
Returns both outgoing and incoming `Contradicts` edges for an engram. Since contradiction is symmetric, this tells you all engrams that contradict or are contradicted by this one.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | ID |

### Returns
    {
        outgoing: [Engram, ...],   // engrams this one contradicts
        incoming: [Engram, ...]    // engrams that contradict this one
    }

### Used By
- Contradiction resolution (consolidation loop)
- Debugging contradiction chains

---

## 8. GetIdentityAlignment

### What It Does
For a given engram, walks the full cognitive chain (`Engram → Recalls → Concept → Defines → Identity`) and returns all identity traits that this engram's concepts define. This is Factor 4 of the resonance formula.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | ID |

### Returns
All Identity nodes reachable from the engram via its recalled concepts.

### Used By
- Resonance Scorer — computes `identity_alignment()` for Factor 4
- Identity crystallization — determines which traits a new engram affects

---

## 9. GetCognitiveChain

### What It Does
The full cognitive trace for an engram: returns the engram itself, its concepts, and the identity traits those concepts define — all in one query with property projection for clean output.

### Parameters
| Param | Type |
|---|---|
| `engram_id` | ID |

### Returns
    {
        engram:     { episode_raw, engram_type, emotional_charge, importance_score },
        concepts:   [{ name, confidence }, ...],
        identities: [{ trait, value, stability }, ...]
    }

### Used By
- Resonance Scorer — full context for scoring
- Debugging and visualization — "show me the full cognitive picture of this memory"
- `Pipeline.update_identity_from_experience()` — updates identity traits based on engram impact

---

## Read Query Call Patterns

### Retrieval Flow
```
Pipeline.retrieve_memories(query):
  1. SearchEngramsFull(query, limit, ...)          → candidates (uses SearchV)
  2. GetAllIdentity()                               → identity traits
  3. For each top candidate:
       GetCognitiveChain(engram_id)                 → alignment data
       Compute resonance in Rust
  4. ActivateEngram(top_candidates)
```

### Consolidation Flow
```
Pipeline.consolidate():
  1. GetEngramsForDecayCheck("episodic")            → candidates for decay
  2. For each engram:
       Compute retention in Rust
       CompressEngram / PruneEngram / PromoteToSemantic
  3. CheckMitosisCandidates(min_count)              → overloaded concepts
  4. For each concept:
       GetEngramsRecallingConcept(concept_id)        → engram embeddings
       Cluster in Rust → split concept
  5. Detect contradictions batch scan
```
