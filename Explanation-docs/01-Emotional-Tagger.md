# Emotional Tagger

## Trigger

**Fires at ingestion time** — immediately when new raw memory text enters the system, before any other ML model. This is step 1 in the ingestion pipeline.

Triggered by: `Pipeline.ingest_memory()` in `pipeline.rs`.

---

## What It Does

Analyzes the raw text of a memory and assigns an **emotional valence** on a continuous scale from **-1.0** (strongly negative) to **+1.0** (strongly positive). A value of 0.0 means neutral.

---

## DB Data It Reads

```
QUERY: (none — processes raw text before any DB write)
```

| Field | Source | Type |
|---|---|---|
| `episode_raw` | Function parameter (raw user input) | `String` |

The emotional tagger runs **before** the engram is stored. It operates on the raw text string passed to the ingestion function. No DB read needed.

---

## DB Data It Writes

```
QUERY: CreateFullEngram
```

| Field Written | Node | Type |
|---|---|---|
| `emotional_charge` | `N::Engram` | `F32` (range: -1.0 to +1.0) |

This value becomes a permanent field on the engram node and is used later by:
- **Resonance Scorer** — `(1 + |emotional_charge|)` amplifies recall of emotional memories
- **Contradiction Detector** — opposite-sign emotions on the same concept trigger contradiction edges
- **Consolidation Loop** — highly emotional engrams resist decay (indirectly, via importance correlation)

---

## How It Works

1. Raw text arrives: *"My colleague publicly humiliated me in the meeting."*
2. The model classifies the emotional valence of the text
3. Returns a float: `-0.85` (strong negative)
4. This value is passed to `CreateFullEngram` as the `emotional_charge` parameter

---

## Real Examples

| Input Text | Output | Why |
|---|---|---|
| "My colleague publicly humiliated me in the meeting." | `-0.85` | Humiliation, social pain, workplace |
| "I finally solved the bug that's been haunting me for weeks." | `+0.72` | Relief, achievement, closure |
| "The 15:00 bus was 4 minutes late today." | `-0.05` | Mild inconvenience, no emotional weight |
| "We signed the biggest client in company history." | `+0.95` | Major success, excitement |
| "The quarterly report shows flat growth again." | `-0.30` | Disappointment, concern, not catastrophic |
| "I updated the README with the new setup instructions." | `0.00` | Purely factual, no emotion |

---

## Recommended Approach: LLM-based Scoring

> **This is the recommended approach.** Emotional tagging needs to understand sarcasm, context, and nuance. LLM is the only option that handles all three.

### Why LLM?

| Feature | Why It Matters |
|---------|----------------|
| **Sarcasm detection** | "Oh great, another meeting" → -0.6 (not +0.8) |
| **Context understanding** | "I finally solved the bug" → positive relief, not just "bug = negative" |
| **Nuance** | "Bittersweet farewell" → mixed, not just positive or negative |
| **Emotional weight** | Understands humiliation vs disappointment vs mild inconvenience |

### Implementation

```
Prompt: "Score the emotional valence of this text from -1.0 (very negative)
         to +1.0 (very positive). Return only the number.

         Text: {episode_raw}"
```

```rust
/// Score emotional valence using LLM
pub async fn tag_emotion(
    text: &str,
) -> Result<f64, Box<dyn std::error::Error>> {
    let prompt = format!(
        "Score the emotional valence of this text from -1.0 (very negative) \
         to +1.0 (very positive). Return only the number.\n\n\
         Text: {}",
        text
    );

    // Call LLM API and parse float response
    let response = llm_client.chat().create(/* ... */).await?;
    let score: f64 = response.trim().parse()?;

    Ok(score.clamp(-1.0, 1.0))
}
```

---

## Edge Cases Handled

| Case | Behavior |
|---|---|
| Empty string | Return `0.0` |
| Non-English text | Depends on model capability; LLM handles, lexicon may fail |
| Sarcasm: "Oh great, another meeting." | Lexicon says +0.8 (word "great"). LLM says -0.6. Choose LLM. |
| Mixed emotions: "Bittersweet farewell party" | Returns near-zero or slightly positive/negative depending on model nuance |
| Very short text: "No." | Context-dependent; LLM may flag low confidence |

---

## Implementation

For the actual Rust DSL implementation of queries mentioned in this doc, see:
- [Query Reference](../helixdb-embedded-queries.md)
- [Full Pipeline](../helixdb-embedded-pipeline.rs)
- [Schema Definition](../helixdb-embedded-schema.md)

For how emotional charge is used in other algorithms, see:
- [06-Resonance-Scorer.md](./06-Resonance-Scorer.md) — emotional amplification in CRR
- [04-Contradiction-Detector.md](./04-Contradiction-Detector.md) — opposite emotions trigger contradictions
