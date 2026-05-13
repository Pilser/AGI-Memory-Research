# Importance Scorer

## Trigger

**Fires at ingestion time** — immediately after the Emotional Tagger, step 2 in the ingestion pipeline. Runs on the same raw text before the engram is stored.

Triggered by: `Pipeline.ingest_memory()` in `pipeline.rs`, right after `emotional_tagger.tag_batch()`.

---

## What It Does

Scores how **important** a memory is on a scale from **0.0** (trivial, forgettable) to **1.0** (critical, must never forget). This single value controls the engram's entire lifecycle — whether it survives decay, gets pruned, or gets promoted to permanent semantic memory.

---

## DB Data It Reads

```
QUERY: (none — processes raw text before any DB write)
```

| Field | Source | Type |
|---|---|---|
| `episode_raw` | Function parameter (raw user input) | `String` |

Same as Emotional Tagger — no DB read. Runs purely on the input text.

---

## DB Data It Writes

```
QUERY: CreateFullEngram
```

| Field Written | Node | Type |
|---|---|---|
| `importance_score` | `N::Engram` | `F32` (range: 0.0 to 1.0) |

---

## How It Works

The importance score is the **primary governor** of the engram's lifecycle:

| Retention Range | importance_score | Action |
|---|---|---|
| `retention < 0.05` AND `importance < 0.1` | `0.0 – 0.1` | **PRUNE** — engram is deleted |
| `retention < 0.15` AND `importance < 0.3` | `0.1 – 0.3` | **COMPRESS** — raw text dropped, structure kept |
| Normal decay | `0.3 – 0.7` | **NORMAL** — decays at standard Ebbinghaus rate |
| `retention > 0.8` AND `activation_count > 20` | `0.7 – 1.0` | **PROMOTE** — elevated to semantic memory (compression_level=2) |

Additionally, the CRR retrieval query uses `min_importance` as a filter:
```
SearchEngramsFull(min_importance=0.3) → excludes trivial memories from retrieval
```

---

## Real Examples

| Input Text | Score | Why |
|---|---|---|
| "I had coffee this morning, it was fine." | `0.08` | Routine, no stakes, no novel information |
| "Updated the footer copyright year to 2025." | `0.12` | Mechanical task, low informational value |
| "The client rejected our proposal after 6 months of work." | `0.92` | High stakes, emotional, business-critical information |
| "I learned that Uganda's ICT Hub is attracting VC from Nairobi." | `0.78` | Novel information, worldview-shaping, builds on existing knowledge |
| "Checked email, nothing important." | `0.03` | Noise |
| "The production database went down at 3am, we lost 2 hours of transactions." | `0.96` | Critical incident, must never be forgotten, informs future decisions |

---

## What Makes a Memory "Important" (Scoring Dimensions)

The model should consider these axes:

| Dimension | High Importance | Low Importance |
|---|---|---|
| **Stakes** | Affects money, health, relationships, career | No consequences |
| **Novelty** | New information, never seen before | Repetitive, already known |
| **Emotional weight** | High emotional charge (already scored by Tagger) | Emotionally flat |
| **Decision relevance** | Informs future decisions | No decision value |
| **Relationship to existing knowledge** | Contradicts, extends, or deepens existing concepts | No connection to known concepts |
| **Specificity** | Concrete details, names, numbers, facts | Vague, generic |

---

## Possible Model Types

### Option A: LLM-based (OpenAI, Anthropic, local LLM)

```
Prompt: "You are a memory importance scorer. Rate how important this
         memory is for a personal AI to retain long-term. Consider:
         - Does it contain novel information?
         - Could it inform future decisions?
         - Are the stakes high?
         - Is it specific or generic?
         Return ONLY a float from 0.0 to 1.0.

         Memory: {episode_raw}"
```

- **Pros:** Best understanding of subtle importance cues, adapts to context
- **Cons:** Latency, API cost

### Option B: Custom classifier (ONNX / Candle)

- Binary or regression model fine-tuned on labeled data
- Input features: text embedding + emotional charge from previous step
- Output: 0.0–1.0 float

- **Pros:** Fast, cheap, private, consistent
- **Cons:** Requires labeled training data

### Option C: Heuristic rules + signals

- Combine programmatic signals into a weighted score:
  - Text length (very short = likely trivial)
  - Presence of names, dates, numbers (specificity)
  - Emotional charge magnitude from Tagger (proxy for stakes)
  - Keyword matching against "important topics" list
  - Whether it contains question marks (decisions pending)

- **Pros:** Fastest, no model, transparent
- **Cons:** Brittle, can't understand context

### Option D: Reinforcement from usage

- Start with a moderate default (e.g., 0.5)
- Over time, boost importance of engrams that get **recalled** (high activation_count means the AI kept referencing it)
- Boost importance of engrams whose concepts get **mitosis** (they spawned new knowledge)
- This **retroactively** adjusts importance based on real usage signals

- **Pros:** Self-improving, no labels needed
- **Cons:** Cold-start problem for new engrams

---

## Relationship to Other Models

| Model | Relationship |
|---|---|
| **Emotional Tagger** | Runs *before* this model. High emotional charge often (but not always) correlates with high importance. The importance model can use the tagger's output as an input feature. |
| **Resonance Scorer** | Uses `importance_score` as a **filter threshold** — only engrams above `min_importance` enter the candidate pool. Does NOT use it in the multiplicative formula (importance already controlled via filter). |
| **Consolidation Loop** | Uses `importance_score` to decide prune vs. compress vs. promote. This is the **life-or-death** field for engrams. |
