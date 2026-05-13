# E::SpawnedFrom — Concept → Concept (Parent → Child)

## What It Is

Links a parent concept to a child concept after **memory mitosis**. When an overloaded concept splits into specialized sub-concepts, `SpawnedFrom` edges connect the parent to each child. This builds the **knowledge hierarchy** — general knowledge at the top, specialized knowledge at the leaves.

---

## Schema

    E::SpawnedFrom {
        From: Concept,    // Parent (general)
        To:   Concept     // Child (specialized)
    }

---

## Direction

    Parent Concept ──SpawnedFrom──→ Child Concept

    From: The parent concept (general category, e.g., "databases")
    To:   The child concept (specialization, e.g., "PostgreSQL replication")

To find the parent of a concept, traverse **incoming**: `N<Concept>(child_id)::In<SpawnedFrom>`

---

## When It's Created

| Trigger | Query | Timing |
|---|---|---|
| Memory mitosis | `ConnectSpawn(parent_id, child_id)` | During consolidation, one edge per child concept spawned |

---

## How It's Used

| Use Case | Query |
|---|---|
| **Get children of a concept** | `N<Concept>(parent_id)::In<SpawnedFrom>` |
| **Get parent of a concept** | `N<Concept>(child_id)::Out<SpawnedFrom>` |
| **Get full knowledge tree** | Recursive traversal in application code |
| **Search with hierarchy** | When querying "databases," include children by traversing SpawnedFrom edges |
| **Trace knowledge evolution** | See how concepts differentiate over time |

---

## Example Hierarchy

    "technology"  (source_count: 200+)
       ├── SpawnedFrom → "databases"  (source_count: 80)
       │      ├── SpawnedFrom → "SQL databases"
       │      │      ├── SpawnedFrom → "PostgreSQL"
       │      │      │      ├── SpawnedFrom → "PostgreSQL replication"
       │      │      │      └── SpawnedFrom → "PostgreSQL query planning"
       │      │      └── SpawnedFrom → "MySQL"
       │      └── SpawnedFrom → "NoSQL databases"
       │             ├── SpawnedFrom → "MongoDB"
       │             └── SpawnedFrom → "Redis"
       └── SpawnedFrom → "programming languages"  (source_count: 120)
              ├── SpawnedFrom → "Rust"
              │      ├── SpawnedFrom → "Rust async programming"
              │      └── SpawnedFrom → "Rust memory management"
              └── SpawnedFrom → "Python"

This tree is not designed upfront — it **emerges** from the mitosis process as the AI accumulates experience.

---

## Relationship to Engrams

After mitosis, engrams are reassigned from parent to child concepts:

    Before: Engram → Recalls → "databases"
    After:  Engram → Recalls → "PostgreSQL replication"  (more specific)

    Parent still keeps: SpawnedFrom → "PostgreSQL replication"
