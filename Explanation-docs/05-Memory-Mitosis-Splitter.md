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
2. Runs clustering (K-means, DBSCAN, or HDBSCAN) on those embeddings
3. Each cluster becomes a new **child `Concept` node** with a name generated from the cluster's representative engrams
4. Creates `SpawnedFrom` edges from the parent concept to each child
5. Reassigns engrams to the nearest child concept (updates `Recalls` and `AbstractsTo` edges)

The parent concept remains — it serves as the **general category**. Children are the **specializations**.

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

Then for each candidate concept, we read its connected engrams and their vectors:

```
QUERY: GetEngramsRecallingConcept(concept_id)
   → then for each engram: GetEngramWithVector(engram_id)
```

| Field Read | Source | Type | Purpose |
|---|---|---|---|
| `ID` | `N::Engram` | `ID` | Identify the engram |
| `episode_raw` | `N::Engram` | `String` | For generating child concept names |
| Embedding | `V::EngramVector` via `HasVector` edge | `[F64]` | For clustering in vector space |

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

```
For concept "databases" (47 engrams):

Retrieve all 47 engram embeddings (1536-dim vectors)
Run HDBSCAN or K-means:

Cluster 0 (18 engrams): postgres, WAL, replication, vacuum, pg_dump
  → Centroid is close to: "PostgreSQL administration"

Cluster 1 (15 engrams): mongodb, documents, aggregation, collections
  → Centroid is close to: "NoSQL document databases"

Cluster 2 (14 engrams): indexes, EXPLAIN, query plans, B-tree, GIN
  → Centroid is close to: "database query optimization"

Outliers (3 engrams): mixed topics, low density
  → Stay attached to parent concept "databases"
```

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

## Possible Model Types

### Option A: Embedding + HDBSCAN (Rust — linfa-clustering or hdbscan crate)

```rust
use linfa_clustering::Dbscan;

let embeddings: Vec<Vec<f64>> = engrams.iter()
    .map(|e| e.vector.clone())
    .collect();

let clustered = Dbscan::params(min_points)
    .transform(&embeddings);
```

- **Pros:** No LLM needed for clustering, fast, deterministic, handles noise/outliers
- **Cons:** Only clusters embeddings — still need LLM for naming child concepts
- **Best for:** The clustering step (always use this)

### Option B: LLM for naming (OpenAI, Anthropic, local)

```
Prompt: "Given these {n} memory snippets about '{parent_concept_name}',
         generate a concise, specific concept name for this cluster.

         Snippets:
         {snippet_1}
         {snippet_2}
         ...

         Return JSON: {name, confidence}"
```

- **Pros:** Generates human-readable, meaningful names
- **Cons:** API cost, but only called once per cluster (not per engram)
- **Best for:** The naming step

### Option C: Centroid-based naming

- Find the engram whose embedding is closest to the cluster centroid
- Extract key phrases from that engram's `episode_raw` using the Concept Extractor
- Use the top extracted concept as the child name

- **Pros:** No additional API calls, reuses existing Concept Extractor
- **Cons:** May produce awkward names if the most-central engram isn't representative
- **Best for:** Privacy-sensitive deployments, offline mode

### Option D: Hierarchical clustering with auto-naming

- Use agglomerative clustering to build a full hierarchy
- Name each level using the same LLM approach
- This replaces the threshold-based trigger with a natural hierarchy

- **Pros:** More natural knowledge structure
- **Cons:** More complex, harder to decide when to stop splitting
- **Best for:** Advanced deployments

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
