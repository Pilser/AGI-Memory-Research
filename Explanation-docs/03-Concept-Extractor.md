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
QUERY: CreateConcept(name, confidence)
QUERY: ConnectRecalls(engram_id, concept_id)
QUERY: ConnectAbstraction(engram_id, concept_id)
QUERY: UpdateConceptSourceCount(concept_id)
```

| Written | Node/Edge | Fields |
|---|---|---|
| New `Concept` node | `N::Concept` | `name`, `confidence`, `formation_date`, `source_count` |
| `Recalls` edge | `E::Recalls` | `From: Engram, To: Concept` |
| `AbstractsTo` edge | `E::AbstractsTo` | `From: Engram, To: Concept` |
| Updated `source_count` | `N::Concept` | Incremented count of linked engrams |

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

## Possible Model Types

### Option A: LLM with structured output (OpenAI, Anthropic)

```
Prompt: "Extract key concepts, entities, and topics from this memory.
         Return a JSON array of objects with 'name' and 'confidence' fields.

         Rules:
         - Each concept should be a specific noun phrase (2-6 words)
         - Avoid overly generic terms ('stuff', 'things')
         - Include technical terms, proper nouns, and domain concepts
         - Confidence should reflect how clearly the concept is stated

         Memory: {episode_raw}"

Response model (Pydantic/JSON Schema):
{
  "concepts": [
    {"name": "Uganda ICT Hub", "confidence": 0.95},
    ...
  ]
}
```

- **Pros:** Best quality, handles implicit concepts, understands context
- **Cons:** Latency, cost, requires structured output parsing
- **Best for:** Production, when concept quality is critical

### Option B: NER + Keyphrase Extraction

- Use a Named Entity Recognition model (spaCy, HuggingFace NER)
- Combine with keyphrase extraction (YAKE, TextRank, KeyBERT)
- Merge and deduplicate

- **Pros:** Fast, deterministic, good for proper nouns
- **Cons:** Misses abstract concepts ("donor dependency narrative"), no confidence score
- **Best for:** High-throughput, when concepts are mostly named entities

### Option C: Local LLM (llama.cpp, mistral.rs, Ollama)

- Run a small LLM (Llama 3 8B, Mistral 7B, Phi-3) locally
- Same prompt as Option A but fully local

- **Pros:** Privacy, no API cost, decent quality
- **Cons:** GPU/CPU requirements, slower than NER
- **Best for:** Privacy-sensitive deployments, offline operation

### Option D: Embedding clustering (batch, offline)

- Embed all unprocessed engrams
- Cluster embeddings (HDBSCAN)
- Each cluster centroid → concept
- This is a **retroactive** approach (not per-engram)

- **Pros:** Finds implicit concepts that NER/LLM miss
- **Cons:** Batch only, no real-time concept creation
- **Best for:** Cold-start population of the concept graph from existing data

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
