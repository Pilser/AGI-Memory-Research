# Memory Mitosis Splitter

## Trigger

**Fires during the consolidation cycle** only — never at ingestion time. This is a background batch process that runs between user sessions (the AI's "sleep" cycle). It checks all concepts for overload and splits those that exceed the `source_count` threshold.

Triggered by: `Pipeline.consolidate()` in `pipeline.rs`, via `check_mitosis()`.

Also can be triggered:
- **After a consolidation cycle** updates concept source_counts
- **Manually** if you want to force knowledge reorganization
- **On a schedule** (e.g., nightly cron job)

---

## What It Does

When a `Concept` node has too many engrams linked to it (high `source_count`), it becomes **overloaded** — too many diverse experiences are feeding into one generic concept. The mitosis splitter:

1. Takes all engram embeddings linked to the overloaded concept
2. Runs **HDBSCAN** clustering on those embeddings (finds dense regions, identifies noise)
3. Each cluster becomes a new **child `Concept` node** with a name generated from the cluster's representative engrams
4. **Noise engrams** (outliers, tangential mentions) stay with the parent concept
5. Creates `SpawnedFrom` edges from the parent concept to each child
6. Reassigns engrams to the nearest child concept (updates `Recalls` and `AbstractsTo` edges)

The parent concept remains — it serves as the **general category**. Children are the **specializations**.

> **How "understanding" works:** HDBSCAN is NOT a language model — it's purely mathematical. The "understanding" that "database admin at grocery store" is NOT about databases comes from the **embedding model** (e.g., `text-embedding-3-small`), which produces vectors where semantically different contexts are far apart. HDBSCAN just finds dense regions of vectors and labels isolated points as noise.

---

## DB Data It Reads

```
QUERY: CheckMitosisCandidates(min_engram_count)
```

| Field Read | Node | Type | Purpose |
|---|---|---|---|
| `ID` | `N::Concept` | `ID` | Candidate concepts to split |
| `name` | `N::Concept` | `String` | Parent concept name |
| `source_count` | `N::Concept` | `U32` | How many engrams feed this concept |

Then for each candidate concept, we read its connected engrams (with embeddings directly on the node):

```
QUERY: get_engrams_recalling_concept_with_embeddings(concept_id)  // Rust DSL
```

| Field Read | Source | Type | Purpose |
|---|---|---|---|
| `$id` | `Engram` | `u64` | Identify the engram |
| `episode_raw` | `Engram` | `String` | For generating child concept names |
| `embedding` | `Engram` (directly on node) | `Vec<f32>` | For clustering in vector space |

**Key simplification:** Embeddings are stored directly on Engram nodes. No separate `EngramVector` node or `HasVector` edge traversal needed. The query returns embeddings in a single step.

---

## DB Data It Writes

```
QUERY: CreateConcept(name, confidence)
QUERY: ConnectSpawn(parent_id, child_id)
QUERY: Drop old Recalls edge, create new ConnectRecalls(engram_id, child_concept_id)
QUERY: Drop old AbstractsTo edge, create new ConnectAbstraction(engram_id, child_concept_id)
```

| Written | Node/Edge | Fields |
|---|---|---|
| Child `Concept` nodes | `N::Concept` | `name`, `confidence`, `formation_date`, `source_count` |
| `SpawnedFrom` edges | `E::SpawnedFrom` | `From: parent_Concept, To: child_Concept` |
| Updated `Recalls` edges | `E::Recalls` | Old edges dropped, new edges created to child concepts |
| Updated `AbstractsTo` edges | `E::AbstractsTo` | Same — redirected to child concepts |

---

## How It Works

### Step 1: Detect Overloaded Concepts

```
Query: CheckMitosisCandidates(min_engram_count=10)

Returns concepts where source_count ≥ 10:
  "databases" → 47 engrams
  "machine learning" → 34 engrams
  "coffee" → 11 engrams
```

### Step 2: Cluster Engram Embeddings

> **How it works:** The embedding model (e.g., `text-embedding-3-small`) converts each engram's text into a 1536-dim vector that captures MEANING (not just words). HDBSCAN is a purely mathematical algorithm that finds dense regions of vectors — it has zero understanding of words. The "understanding" comes from the embedding model.

```
For concept "databases" (47 engrams):

1. Fetch embeddings (vectors) for all 47 engrams
   - Technical DB vectors cluster together (close in vector space)
   - "Database admin at grocery store" vector is FAR away (different context)
   - "Database of my mind" vector is FAR away (metaphorical)

2. Run HDBSCAN on the vectors:

Cluster 0 (18 engrams): postgres, WAL, replication, vacuum, pg_dump
  → Dense region in vector space → "PostgreSQL administration"

Cluster 1 (15 engrams): mongodb, documents, aggregation, collections
  → Dense region in vector space → "NoSQL document databases"

Cluster 2 (14 engrams): indexes, EXPLAIN, query plans, B-tree, GIN
  → Dense region in vector space → "database query optimization"

Noise (3 engrams): "database admin at grocery store", "database of my mind", etc.
  → Isolated points far from any dense region → stay with parent "databases"
```

**Why HDBSCAN over K-means:** HDBSCAN identifies noise (points that don't belong to any cluster). K-means forces every point into a cluster. For memory, noise handling is critical — tangential mentions and metaphors shouldn't form their own child concepts.

### Step 3: Generate Child Concept Names

For each cluster, use an LLM to generate a name from the engram texts:

```
Prompt: "Given these memory snippets about a shared topic,
         generate a specific, concise concept name (2-6 words):

         Snippets: [cluster engram texts...]

         Return: {name, confidence}"
```

### Step 4: Create Child Concepts + Edges

```
For each cluster:
  Create Concept{name: "PostgreSQL administration", confidence: 0.88}
  Create SpawnedFrom edge: "databases" → "PostgreSQL administration"

  For each engram in the cluster:
    Drop Recalls edge: engram → "databases"
    Create Recalls edge: engram → "PostgreSQL administration"
    Drop AbstractsTo edge: engram → "databases"
    Create AbstractsTo edge: engram → "PostgreSQL administration"

Update source_count on:
  "databases" (parent, now reduced by 18)
  "PostgreSQL administration" (child, set to 18)
```

### Optional Enhancement: Store Centroid on Child Concepts

> **Optional enhancement** — improves future mitosis decisions without degrading anything.
> After creating child concepts, compute and store centroid embeddings on them.

```
For each child concept:
  centroid = average of all assigned engrams' embeddings
  Update Concept set embedding = centroid
```

**Benefits:**
- **Faster future mitosis:** Compare concept centroids instead of re-clustering all engrams
- **Concept-level search:** Can search concepts directly by semantic similarity
- **Drift detection:** Compare child centroid to parent centroid to see if they're diverging

**Cost:** Extra computation after mitosis (compute centroid, store on concept)

---

## Real Example: Full Mitosis Journey

```
START:
  Concept "databases"
    source_count: 6
    → Below threshold. No action.

LATER:
  Concept "databases"
    source_count: 14
    → Above threshold (≥10)
    → Clustering shows 2 distinct groups

  Child 1: "PostgreSQL internals" (8 engrams)
  Child 2: "database performance tuning" (4 engrams)
  Parent "databases" retains 2 outlier engrams

MUCH LATER:
  Concept "PostgreSQL internals"
    source_count: 24
    → Above threshold again!

  Child 1a: "PostgreSQL WAL and replication" (10 engrams)
  Child 1b: "PostgreSQL vacuum and maintenance" (8 engrams)
  Child 1c: "PostgreSQL query planning" (6 engrams)

  Concept "database performance tuning"
    source_count: 19

  Child 2a: "SQL query optimization" (9 engrams)
  Child 2b: "database indexing strategies" (10 engrams)

RESULT — Knowledge Fractal:
  databases
    ├── PostgreSQL internals
    │    ├── PostgreSQL WAL and replication
    │    ├── PostgreSQL vacuum and maintenance
    │    └── PostgreSQL query planning
    ├── database performance tuning
    │    ├── SQL query optimization
    │    └── database indexing strategies
    └── [outlier engrams]
```

The AI's knowledge naturally differentiates from general to specific — just like human expertise develops from "I know about databases" to "I understand the nuances of PostgreSQL WAL segment sizing."

---

## Recommended Approach

### Clustering: HDBSCAN (Rust `hdbscan-rs`)

> **HDBSCAN is NOT a language model.** It's a purely mathematical algorithm that finds dense regions of vectors. The "understanding" of which engrams are about the same topic comes from the EMBEDDING MODEL (e.g., `text-embedding-3-small`), not HDBSCAN.

```rust
use hdbscan::Hdbscan;

let embeddings: Vec<Vec<f32>> = engrams.iter()
    .map(|e| e.embedding.clone())  // Embedding stored directly on Engram node
    .collect();

let clusterer = Hdbscan::default();
let labels = clusterer.cluster(&embeddings);

// labels[i] = cluster ID, or -1 for noise
// Noise engrams stay with parent concept (not forced into a child)
```

- **Pros:** Handles noise (outliers stay with parent), finds natural number of clusters, no LLM needed, fast
- **Cons:** Still need naming strategy for child concepts (see below)
- **Best for:** The clustering step — **this is the recommended approach**

### Naming: LLM (Recommended — Best Quality)

> **This is the recommended naming approach.** Names are permanent parts of your knowledge structure — worth doing right. LLM produces human-readable, meaningful names that centroid-based can't match.

```
Prompt: "Given these {n} memory snippets about '{parent_concept_name}',
         generate a concise, specific concept name (2-6 words):

         Snippets:
         {snippet_1}
         {snippet_2}
         ...

         Return JSON: {name, confidence}"
```

| Approach | Name Quality | Cost | Privacy | Recommendation |
|----------|-------------|------|---------|----------------|
| **LLM naming** | **Best** — human-readable, meaningful | Minimal (once per cluster) | Requires API | **Recommended** |
| Centroid-based | Good — can be awkward | Free | Fully local | Fallback for offline |

**Why LLM:** Names are permanent — they're used for search, traversal, and future mitosis. The cost is negligible (one LLM call per cluster, rare event during consolidation), but the quality difference is significant. "PostgreSQL WAL and replication" (LLM) vs "postgres WAL replication vacuum" (centroid).

**Fallback:** Use centroid-based naming only if you need fully offline operation.

---

## Thresholds and Configuration

| Parameter | Default | Meaning |
|---|---|---|
| `min_engram_count` | 10 | Minimum source_count to trigger mitosis |
| `max_engram_count` | 50 | If above this, always split regardless of cluster quality |
| `min_cluster_size` | 3 | Minimum engrams per child cluster (smaller = noise, stays with parent) |
| `max_children` | 5 | Maximum child concepts per mitosis event (prevents fragmentation) |
| `min_child_confidence` | 0.5 | Minimum confidence for child concept naming to accept |

---

## What Happens to the Parent?

The parent concept is **not deleted**. It:
- Keeps outlier engrams (those that don't fit any cluster)
- Maintains `SpawnedFrom` edges to its children
- Gets its `source_count` recalculated (should be reduced)
- Serves as a **category node** for search and traversal — querying "databases" can traverse `SpawnedFrom` outgoing edges to include all children

---

## Edge Cases

| Case | Behavior |
|---|---|
| All engrams in one cluster | No split. Concept doesn't need mitosis yet. |
| Too many clusters (> max_children) | Keep top N clusters by size. Remainder stay with parent. |
| Cluster too small (< min_cluster_size) | Merge into parent. Don't create a child for 1-2 engrams. |
| Parent had contradictions | Children inherit relevant `Contradicts` edges. Re-evaluate contradictions at child level. |
| Concept was just created | Skip — mitosis only applies to concepts with real accumulation, not brand-new ones. |

---

## Implementation

For the actual Rust DSL implementation of queries mentioned in this doc, see:
- [Query Reference](../helixdb-embedded-queries.md)
- [Full Pipeline](../helixdb-embedded-pipeline.rs)
- [Schema Definition](../helixdb-embedded-schema.md)
