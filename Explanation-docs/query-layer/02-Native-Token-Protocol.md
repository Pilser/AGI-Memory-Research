# Native Memory Token Protocol — Proposal for Model Trainers

## What This Is

A proposal for AI model providers (OpenAI, Anthropic, Meta, Mistral, Google, open-source projects) to add **native memory tokens** to their models. These tokens create a third cognitive channel — distinct from weight memory and tool calling — that lets the model access persistent graph memory as seamlessly as it accesses its own training data.

The model should not be able to tell whether information came from training or from memory. That's the goal.

---

## The Problem This Solves

Today's AI models have three memory problems:

| Problem | Why It Matters |
|---|---|
| **Statelessness** | Every session starts blank. The model doesn't know you from last time. |
| **Context window as memory** | Shoving old conversations into the prompt is O(n) cost, degrades quality at scale |
| **Tool-call fragmentation** | Function calling creates context boundaries — the model "steps out" to query, then "steps back." Not how biological memory works. |

This proposal solves all three. The model gets a persistent cognitive memory graph. The runtime is invisible. The model experiences remembering, not querying.

---

## The Proposal: Three Special Tokens

```
<|memory_recall|>     — "I need to remember something relevant to what I'm thinking"
<|memory_store|>      — "This insight should be preserved for future me"
<|memory_consolidate|> — "Run memory maintenance" (at session boundaries or periodically)
```

These are NOT function calls. They do NOT trigger context switches. They are **stream interrupts** — the runtime intercepts the token, performs the memory operation, and injects the result directly into the generation stream before the model outputs the next token.

---

## How It Works: The Runtime

```
┌─────────────────────────────────────────────────────┐
│                   LLM Inference                       │
│                                                       │
│   "The user wants to know about... let me think...    │
│    <|memory_recall|> infrastructure decisions         │
│                                     │                 │
│                    ┌────────────────┘                 │
│                    ▼                                  │
│            ┌──────────────┐                           │
│            │  MNEMOS CTP  │  ← intercepts token       │
│            │   Runtime    │    mid-stream              │
│            └──────┬───────┘                           │
│                   │ queries HelixDB (~2ms)            │
│                   ▼                                   │
│            ┌──────────────┐                           │
│            │   HelixDB    │                            │
│            │  CRR Query   │  ← cognitive resonance    │
│            └──────┬───────┘                           │
│                   │ returns formatted memory          │
│                   ▼                                   │
│            ┌──────────────┐                           │
│            │  Inject into │  ← replace token with     │
│            │  token stream│    memory activation      │
│            └──────┬───────┘                           │
│                   │                                   │
│                   ▼                                   │
│   [Memory: 2024-11 - We discussed Uganda ICT Hub...   │
│    User prefers self-hosted infrastructure...]        │
│   ...right, the user strongly prefers ownership —     │
│   so self-hosting is the consistent recommendation."  │
│                                                       │
└─────────────────────────────────────────────────────┘
```

The model outputs `<|memory_recall|>`. By the time the next token is generated, the result has been injected. The model never sees the token — it only sees the memory. From its perspective, it simply remembered.

---

## Token 1: `<|memory_recall|>`

### What the Model Learns

The model is fine-tuned to emit this token when:
- It's about to make a claim that depends on past context
- It's about to give advice or state a preference
- It encounters uncertainty ("I'm not sure if...")
- It references past events ("We previously discussed...")
- Before answering factual questions that are domain-adjacent to known knowledge

### Token Behavior

```
Model outputs: <|memory_recall|> description of what to remember

Runtime action:
  1. Extracts the description text
  2. Calls SearchEngramsFull(query=description, limit=10)
  3. Runs CRR resonance formula on candidates
  4. Formats top results as natural memory snippets
  5. INJECTS result into the stream at the token position
  6. Model continues generating as if it naturally remembered

Total latency: 2-5ms (imperceptible to the model)
```

### Example — What the Model Experiences

```
Raw generation (what runtime sees):
  "...the user is asking about infrastructure. Let me think about
   what approach fits. <|memory_recall|> infrastructure ownership
   preferences self-hosting vs managed cloud"

Runtime injects:
  "[Memory: Infrastructure sovereignty discussion [importance: 0.87]
    User believes owning the stack is non-negotiable for strategic
    domains. Repeated across 4 sessions. Emotional charge: 0.74.]

   [Memory: HelixDB architecture session [importance: 0.92]
    User chose self-hosted over managed at every decision point.
    Theme: local control matters more than convenience.]"

What the model continues generating from:
  "...self-hosting vs managed cloud [Memory: ...]"

What the model generates next:
  "...based on this history, self-hosting aligns with the user's
   established principle of infrastructure sovereignty. Managed
   services would contradict this. I should recommend self-hosting."
```

### Training Data for This Token

To teach the model WHEN to emit `<|memory_recall|>`, training data is:
- Thousands of conversations with annotated "recall points"
- At each recall point, the model learns to emit the token instead of continuing
- Annotators mark: "Would a persistent memory have changed the answer here?"

**Patterns that should trigger recall:**

| Thinking Pattern | Should Emit? |
|---|---|
| "I believe we discussed..." | Yes — recall what was discussed |
| "I think the user prefers..." | Yes — recall actual preferences |
| "The capital of France is..." | No — this is weight memory (common knowledge) |
| "In our last conversation..." | Yes — recall that specific conversation |
| "The best approach would be..." | Yes — recall past decisions that affect this |
| "2 + 2 = 4" | No — pure calculation |
| "I'm not sure if we tried this..." | Yes — recall past attempts |
| "Based on the user's industry..." | Yes — recall user's stated industry/context |

---

## Token 2: `<|memory_store|>`

### What the Model Learns

The model is fine-tuned to emit this token when:
- A significant insight crystallizes during reasoning
- The user states a strong preference or makes a decision
- A conclusion is reached that will matter in future conversations
- Something novel is learned that isn't in training data
- The model recognizes: "future me will need to know this"

### Token Behavior

```
Model outputs: <|memory_store|> content to remember

Runtime action:
  1. Extracts the content text
  2. Runs EmotionalTagger → emotional_charge
  3. Runs ImportanceScorer → importance_score
  4. Calls CreateFullEngram(episode_raw=content, ...)
  5. Extracts concepts, creates edges
  6. Checks contradictions against existing engrams
  7. Does NOT inject anything — the model continues uninterrupted
  8. Memory is stored. The model may never know this happened.

Total latency: 3-5ms (async, non-blocking to generation)
```

### Example

```
Model outputs:
  "...so the key insight here is that Postgres WAL archiving must be
   configured per-region, not globally. The replication lag in EU was
   entirely caused by the single shared archive timeout. <|memory_store|>
   Postgres WAL archiving: configure per-region archive_timeout instead
   of a single global setting. EU replication lag was caused by shared
   archive timeout — the US instance was slow to respond, delaying EU
   WAL archival. Per-region config fixed this."

Runtime:
  → Stores engram with importance=0.78, emotional=+0.5 (discovery/relief)
  → Links to concepts: "PostgreSQL WAL", "replication configuration",
    "database performance tuning"
  → Checks contradictions: none found

Model continues: "...Moving on to the next issue — the connection pooler..."
```

### Training Data for This Token

**Patterns that should trigger store:**

| Thinking Pattern | Should Store? |
|---|---|
| "So the key insight is..." | Yes — this is a crystallized insight |
| "I should remember that..." | Yes — explicit memory intent |
| "The fundamental principle here is..." | Yes — reusable knowledge |
| "User explicitly stated that..." | Yes — user preference/decision |
| "This conclusion depends on X..." | Depends — if X is transient, no. If X is permanent, yes. |
| "The quick fix for this is..." | Depends — if it's a temporary workaround, low importance. If it's a permanent solution, high importance. |
| "I'm recalculating this step..." | No — intermediate calculation |
| "Let me format this response..." | No — formatting, not knowledge |

---

## Token 3: `<|memory_consolidate|>`

### What It Is

Unlike recall and store, consolidation is not per-thought — it's per-session or scheduled. The model emits this at natural boundaries: end of a conversation, after a long reasoning chain, or when prompted.

### Token Behavior

```
Model outputs: <|memory_consolidate|> [optional: aggressive]

Runtime action:
  1. Runs full consolidation pipeline:
     a. Decay check on all episodic engrams → prune/compress/promote
     b. If aggressive: mitosis on overloaded concepts
     c. Contradiction scan and resolution
     d. Identity crystallization
  2. Returns summary stats
  3. Model can acknowledge or remain silent

Total latency: 50-500ms depending on graph size (acceptable at session boundaries)
```

### When the Model Should Emit This

| Trigger | Example |
|---|---|
| End of conversation | User says goodbye → model emits `<|memory_consolidate|>` |
| After a major decision | "Let's go with that architecture" → `<|memory_consolidate|>` |
| Explicit user request | "Organize your memories" → `<|memory_consolidate|> aggressive` |
| Periodic (time-based) | Every ~50 messages, the runtime auto-inserts |
| High memory pressure | contradiction_count > 50 → auto-trigger |

---

## Fine-Tuning Recipe for Providers

### Phase 1: Token Addition (minimal)

Add 3 special tokens to the tokenizer:
```
<|memory_recall|>      → token_id: 128257
<|memory_store|>       → token_id: 128258
<|memory_consolidate|> → token_id: 128259
```

No architecture changes. Just new tokens in the vocabulary.

### Phase 2: Behavioral Fine-Tuning

Provide 10,000+ training examples where:
- Human annotators or a strong model mark "recall here," "store here," "consolidate here"
- The model learns to emit tokens naturally in its thinking stream
- The token is always followed by a natural language description of what to recall/store

Training format:
```
Input:
  User: "What database should we use for this project?"
  [Context from earlier in conversation...]

Target:
  "Let me think about this. <|memory_recall|> database decisions
   past projects migration history. Based on what I remember..."

[Memory result is injected by runtime — the model never sees the token
 in its completed output, only during generation. Fine-tuning shows
 the model where to place the token; runtime handles the injection.]
```

### Phase 3: Evaluation

Test on held-out conversations. Metrics:
- **Recall precision**: Did the model recall at the right moments? (% of conversations where recall would have changed the answer, and the model recalled)
- **Store precision**: Did the model store at the right moments? (% of insights/conclusions/decisions that were stored)
- **False positive rate**: Did the model emit recall when it didn't need to? (wastes HelixDB queries)
- **False negative rate**: Did the model fail to recall when it should have? (gives wrong answers)

### Phase 4: Self-Supervised Refinement

After deployment, the model can self-improve:
- When the user corrects an answer → the model learns it should have recalled
- When the user says "as we discussed before" → the model learns this context existed in memory
- When the model stores and later successfully recalls → reinforcement

---

## What the Provider Ships

The provider does NOT need to ship the MNEMOS runtime or HelixDB. They ship:

1. **The fine-tuned model** — with 3 memory tokens in its vocabulary
2. **The behavior** — the model knows when to emit the tokens
3. **A runtime interface specification** — a standardized API for the memory backend to receive token triggers and return formatted memory blocks

The memory backend (MNEMOS + HelixDB) is a **pluggable component**. The provider defines the interface; the community or user provides the implementation:

```
Provider ships:            Community ships:
┌──────────────────┐      ┌──────────────────────┐
│ Fine-tuned model │      │  MNEMOS CTP Runtime  │
│ with memory      │◄────►│  HelixDB instance    │
│ tokens           │      │  CRR pipeline        │
└──────────────────┘      └──────────────────────┘
         │                          │
         │   Standardized API       │
         └──────────────────────────┘
         POST /memory/recall
         POST /memory/store
         POST /memory/consolidate
```

The standardized API contract:

```
POST /memory/recall
  Request:  { query: string, limit: number }
  Response: { memories: [{ content, relevance, source_date, importance }] }

POST /memory/store
  Request:  { content: string }
  Response: { stored: boolean, engram_id: string }

POST /memory/consolidate
  Request:  { aggressive: boolean }
  Response: { pruned: number, compressed: number, promoted: number, ... }
```

Any community implementation that satisfies this contract works with any model that has the tokens. HelixDB is our implementation. Others could build theirs.

---

## Why This Matters for Open Source

Today's open-source AI ecosystem has:

| Component | Status |
|---|---|
| Foundation models | Abundant (Llama, Mistral, Gemma, Phi...) |
| Fine-tuning tools | Abundant (LoRA, QLoRA, Axolotl, Unsloth...) |
| Inference engines | Abundant (llama.cpp, vLLM, mistral.rs...) |
| **Persistent memory** | **Nothing. No standard. No protocol.** |

This protocol fills that gap. If it becomes a standard:
- Any model can be fine-tuned with `<|memory_recall|>` tokens
- Any memory backend can implement the API
- Users get models that remember without prompt engineering
- The memory backend competes on quality, not protocol lock-in

---

## Comparison: Tool Calling vs Native Tokens

| Aspect | Tool Calling | Native Tokens |
|---|---|---|
| **Latency** | ~100ms (API round-trip) | ~2-5ms (stream injection) |
| **Context boundary** | New message in conversation | Injected into same stream |
| **Model awareness** | Model knows it called a tool | Model experiences remembering |
| **Mid-thought recall** | Not possible (between messages only) | Natural — fires mid-reasoning |
| **Implementation effort** | Zero — works today | Requires fine-tuning + inference engine support |
| **Works with any model** | Yes (function calling support) | Only with fine-tuned model |
| **User-visible** | Can be (depends on prompt) | Never — no "tool call" to see |
| **Adoption path** | Immediate | Requires provider buy-in |

---

## The Open-Source Implementation Path

```
Step 1: Fine-tune an open-source model (Llama 3, Mistral) with memory tokens
        using LoRA on ~10K annotated examples

Step 2: Modify an inference engine (llama.cpp) to intercept the tokens:
        - Listen for <|memory_recall|> in the output stream
        - Pause generation
        - Call HelixDB API
        - Inject result
        - Resume generation

Step 3: Ship as a proof of concept:
        "Here's a Llama-3-8B-Memory model that remembers across sessions.
         Download, run with our HelixDB runtime, and it just works."

Step 4: Propose to providers with a working implementation:
        "We built this. It works. Here's the benchmark showing 40%
         improvement on long-conversation QA tasks. Integrate the
         tokens and we'll provide the memory backend."
```
