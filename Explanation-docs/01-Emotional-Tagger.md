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

## Possible Model Types

### Option A: LLM-based (OpenAI, Anthropic, local LLM)

```
Prompt: "Score the emotional valence of this text from -1.0 (very negative)
         to +1.0 (very positive). Return only the number.

         Text: {episode_raw}"
```

- **Pros:** Highest accuracy, understands nuance, context, sarcasm
- **Cons:** Latency (~200-500ms), API cost if using cloud
- **Best for:** Production system where accuracy matters

### Option B: Custom fine-tuned transformer (ONNX / Candle)

- Start with a pre-trained model (`distilbert-base-uncased`, `roberta-base`)
- Fine-tune on your own labeled emotion data
- Export to ONNX, run via `ort` crate in Rust

- **Pros:** Fast (5-20ms), no API cost, runs locally, full control
- **Cons:** Need labeled training data, lower ceiling than GPT-4
- **Best for:** High-throughput systems, privacy-sensitive deployments

### Option C: Lexicon/heuristic based

- Use a sentiment lexicon (VADER, AFINN, NRC Emotion Lexicon) adapted to Rust
- Count positive/negative words, compute ratio

- **Pros:** Zero latency, no model loading, no dependencies
- **Cons:** Low accuracy, no contextual understanding, sarcasm fails completely
- **Best for:** Prototyping, as fallback, or as baseline to compare against

### Option D: Hybrid

- Lexicon-based for fast first pass
- If confidence is low (e.g., ambiguous text), fall back to LLM

---

## Edge Cases Handled

| Case | Behavior |
|---|---|
| Empty string | Return `0.0` |
| Non-English text | Depends on model capability; LLM handles, lexicon may fail |
| Sarcasm: "Oh great, another meeting." | Lexicon says +0.8 (word "great"). LLM says -0.6. Choose LLM. |
| Mixed emotions: "Bittersweet farewell party" | Returns near-zero or slightly positive/negative depending on model nuance |
| Very short text: "No." | Context-dependent; LLM may flag low confidence |
