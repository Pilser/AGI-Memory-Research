# Concept Extractor

## Trigger

**Fires at ingestion time** — step 3, immediately after the engram is stored in HelixDB (because the engram ID is needed to create the `Recalls` edges linking engram → concept).

Triggered by: `Pipeline.ingest_memory()` in `pipeline.rs`, after `CreateFullEngram` returns the engram ID.

---

## What It Does

Extracts named **concepts, entities, beliefs, and topics** from the raw memory text. Each extracted concept becomes a `Concept` node. Edges are created linking the engram to each concept. If a concept already exists in the DB (same name), it is reused rather than duplicated — and its `source_count` is incremented.

---

## DB Data It Reads

```
QUERY: GetConceptByName(name)
```

| Field Read | Node | Type | Purpose |
|---|---|---|---|
| `name` | `N::Concept` | `String` (indexed) | Check if concept already exists |
| `ID` | `N::Concept` | `ID` | Reuse existing node if found |

Called once per extracted concept before creation, to avoid duplicates.

---

## DB Data It Writes

```
QUERY: create_concept (Rust DSL #[query] function)
QUERY: connect_recalls (Rust DSL #[query] function)
QUERY: connect_abstraction (Rust DSL #[query] function)
QUERY: update_concept_source_count (Rust DSL #[query] function)
```

| Written | Node/Edge | Fields |
|---|---|---|
| New `Concept` node | `Concept` | `name`, `confidence`, `formation_date`, `source_count` |
| `Recalls` edge | `Recalls` | `From: Engram, To: Concept` |
| `AbstractsTo` edge | `AbstractsTo` | `From: Engram, To: Concept` |
| Updated `source_count` | `Concept` | Incremented count of linked engrams |

**Optional enhancement:** Use `connect_recalls_with_embedding` to store relationship embeddings on Recalls edges for dynamic Stimulation Layer weights.

---

## How It Works

1. Raw text: *"The Uganda ICT Hub in Kampala is attracting venture capital from Nairobi-based funds."*
2. Concept Extractor analyzes the text
3. Returns structured concepts with confidence scores
4. For each concept:
   - Check DB: does `Concept{name: "Uganda ICT Hub"}` already exist?
   - If yes → reuse its ID
   - If no → create new `Concept` node
   - Link engram → concept via `Recalls` edge (engram recalls this concept)
   - Link engram → concept via `AbstractsTo` edge (engram contributes to this concept)
   - Update `source_count` on the concept (how many engrams feed into it)

---

## Real Examples

```
Input: "The Uganda ICT Hub in Kampala is attracting venture capital
        from Nairobi-based funds, which challenges the narrative that
        East African tech is donor-dependent."

→ ConceptExtractor → [
    {name: "Uganda ICT Hub",          confidence: 0.95},
    {name: "Kampala tech ecosystem",  confidence: 0.88},
    {name: "Nairobi venture capital", confidence: 0.82},
    {name: "East African tech independence", confidence: 0.76},
    {name: "donor dependency narrative", confidence: 0.70}
]

→ Creates/updates 5 Concept nodes
→ Creates 10 edges (5 Recalls + 5 AbstractsTo)
→ Increments source_count on each concept
```

```
Input: "Finally fixed the Postgres WAL replication lag by adjusting
        max_wal_senders and checkpoint_timeout."

→ ConceptExtractor → [
    {name: "PostgreSQL",              confidence: 0.95},
    {name: "WAL replication",         confidence: 0.90},
    {name: "database tuning",         confidence: 0.82},
    {name: "checkpoint configuration", confidence: 0.78}
]
```

```
Input: "Had coffee."

→ ConceptExtractor → [
    {name: "coffee", confidence: 0.60}
]
```

---

## Why Concepts Matter

Concepts are the **bridge** between raw episodic memory and structured knowledge:

| Layer | What it does | Example |
|---|---|---|
| Engram | Raw experience | "The Uganda ICT Hub just raised $5M..." |
| Concept | Abstracted knowledge | "Uganda ICT Hub" |
| Identity | Stable belief about self | "I know East African tech" |

Concepts enable:
- **Memory Mitosis** — when a concept's `source_count` exceeds a threshold, it spawns specialized child concepts
- **Contradiction Detection** — two engrams about the same concept with opposite emotional charges get flagged
- **Identity Crystallization** — concepts that persist over many engrams become identity traits
- **Semantic Search** — you can search by concept name, not just vector similarity

---

## Recommended Approach: LLM with Structured Output

> **This is the recommended approach.** Concepts become permanent parts of your knowledge structure — worth doing right. LLM extracts abstract concepts that simpler methods miss.

### Why LLM?

| Feature | Why It Matters |
|---------|----------------|
| **Abstract concepts** | Extracts "donor dependency narrative" — NER can't do this |
| **Context understanding** | Knows "database" means different things in different contexts |
| **Confidence scores** | Used for mitosis and contradiction detection |
| **Implicit concepts** | Finds concepts not explicitly named in the text |

### Implementation

```
Prompt: "Extract key concepts, entities, and topics from this memory.
         Return a JSON array of objects with 'name' and 'confidence' fields.

         Rules:
         - Each concept should be a specific noun phrase (2-6 words)
         - Avoid overly generic terms ('stuff', 'things')
         - Include technical terms, proper nouns, and domain concepts
         - Confidence should reflect how clearly the concept is stated

         Memory: {episode_raw}"

Response:
{
  "concepts": [
    {"name": "Uganda ICT Hub", "confidence": 0.95},
    {"name": "Kampala tech ecosystem", "confidence": 0.88},
    {"name": "Nairobi venture capital", "confidence": 0.82},
    {"name": "East African tech independence", "confidence": 0.76},
    {"name": "donor dependency narrative", "confidence": 0.70}
  ]
}
```

```rust
/// Extract concepts from raw text using LLM
pub async fn extract_concepts(
    text: &str,
) -> Result<Vec<ExtractedConcept>, Box<dyn std::error::Error>> {
    let prompt = format!(
        "Extract key concepts, entities, and topics from this memory.
         Return a JSON array of objects with 'name' and 'confidence' fields.

         Rules:
         - Each concept should be a specific noun phrase (2-6 words)
         - Avoid overly generic terms ('stuff', 'things')
         - Include technical terms, proper nouns, and domain concepts
         - Confidence should reflect how clearly the concept is stated

         Memory: {}",
        text
    );

    // Call LLM API and parse structured response
    let response = llm_client.chat().create(/* ... */).await?;
    let concepts: Vec<ExtractedConcept> = parse_concepts(&response)?;

    Ok(concepts)
}
```

---

## Duplicate Detection Strategy

The `GetConceptByName` query prevents duplicate concepts:

```rust
// Before creating a concept, check if it exists
let existing = client.query("GetConceptByName", &json!({
    "name": &extracted.name
})).await?;

if existing exists {
    // Reuse existing concept ID
    concept_id = existing.id;
} else {
    // Create new concept
    let created = client.query("CreateConcept", &json!({
        "name": &extracted.name,
        "confidence": extracted.confidence
    })).await?;
    concept_id = created.id;
}
```

This means the first engram about "Uganda ICT Hub" creates the concept. The 47th engram about it links to the same concept, incrementing its `source_count` to 47 — which may trigger **mitosis**.

---

## Edge Cases

| Case | Behavior |
|---|---|
| No concepts found | Skip concept creation. Engram stored without concept links. |
| 20+ concepts in one text | Cap at a reasonable max (e.g., 10). Trim by confidence score. |
| Concept name too generic | LLM prompt instructs avoiding generic terms. As fallback, filter by minimum word count (≥2 words) or blacklist. |
| Concept name already exists but different meaning | Risk of collision. Mitigated by also comparing engram context. Advanced: use concept embedding + cosine threshold alongside name match. |

---

## Implementation

For the actual Rust DSL implementation of queries mentioned in this doc, see:
- [Query Reference](../helixdb-embedded-queries.md)
- [Full Pipeline](../helixdb-embedded-pipeline.rs)
- [Schema Definition](../helixdb-embedded-schema.md)

For how concepts are used in other algorithms, see:
- [05-Memory-Mitosis-Splitter.md](./05-Memory-Mitosis-Splitter.md) — mitosis uses concepts
- [04-Contradiction-Detector.md](./04-Contradiction-Detector.md) — contradiction detection uses concepts
