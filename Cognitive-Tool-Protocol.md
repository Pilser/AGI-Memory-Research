##   The Cognitive Tool Protocol (CTP)
The core idea: the LLM emits lightweight XML-style tags naturally inside its extended thinking stream, the runtime intercepts them mid-stream, executes against HelixDB, and injects results back — all before the thinking finishes. The LLM never even breaks stride.




---

## The Sliding Window Buffer

This is the foundation everything else sits on. The thinking stream is a continuous river of tokens — the interceptor never touches the LLM itself, it sits on the output pipe. The buffer keeps the last 128 tokens in memory at all times, sliding forward with every new token.

Why 128? Long enough to capture a complete thought unit ("I wonder what we said earlier about infrastructure ownership in Uganda"). Short enough to run the ML classifier cheaply on every stride. The buffer is circular — old tokens fall off the back, new ones come in at the front. The stream is never paused here; this is pure passive listening.

---

## Two Detectors Running in Parallel

**The explicit tag detector** is just a state machine — fast, zero-cost, runs on every single token. It watches for the character sequences `<recall`, `<remember`, `<contradiction_check`. When it sees one opening, it flips into capture mode, accumulating everything until the closing tag. Then it fires the blocking path. This is deterministic, always correct, zero ML involved.

**The ML signal classifier** is the novel piece. It runs every 32 tokens (the stride) on the current 128-token window. It outputs three things: signal type (`RECALL`, `STORE`, or `PASS`), a confidence score, and a memory hint string — a short natural language description of what it thinks should be queried or stored.

The classifier is small on purpose — 50–100M parameters, roughly BERT-class. It doesn't need to understand content deeply. It's trained to recognize thinking patterns that signal memory need. Things like: hedging language before making a claim ("I believe we established that..." → `RECALL`), conclusive language at the end of a reasoning chain ("So the key insight here is..." → `STORE`), uncertainty language ("I'm not sure if this contradicts..." → `RECALL` with contradiction type).

Only signals above 0.8 confidence fire. Below that they're discarded — false positives waste HelixDB queries and pollute the thinking stream with irrelevant activations.

---

## The Two Paths — Blocking vs Async

This is the most important architectural decision in the whole runtime, and it's driven by one question: does the LLM need the memory result before it can continue reasoning?

For explicit tags, the answer is yes. The LLM explicitly stopped to ask for a memory. It would make no sense to let it keep thinking without the answer — the next 200 tokens of reasoning would be built on an incomplete foundation. So the explicit path pauses the outgoing token stream, holds the tokens in a gate buffer, fires the HelixDB CRR query (~2ms), injects the `<memory_activation>` block exactly where the `<recall>` tag was, then releases the gate. The LLM continues as if the memory was there all along. Total pause: 2–5ms. Imperceptible.

For implicit signals, the LLM didn't explicitly ask — the classifier inferred a memory need from the pattern of its thought. The stream keeps flowing. The query fires async. The result goes into a pending injection queue. When the next natural boundary arrives — a period followed by newline, a paragraph break, or a pause token — the pending injection gets woven in. The LLM reads it as context that appeared naturally in its thinking. This path adds zero latency to the stream, but the injected memory arrives slightly "late" — maybe 50–100 tokens after the point that triggered it. That's acceptable because implicit signals are lower confidence anyway.

---

## The ML Classifier — What It Actually Learns

Training data is annotated thinking streams. You run a large LLM on thousands of diverse prompts, capture the full thinking output, then have annotators (or a stronger model) label each 32-token window with: "does this thinking segment signal a memory recall need? A store need? Or neither?"

The patterns it learns to detect:

`RECALL` patterns — uncertainty hedges, past-reference language, incomplete knowledge markers:
```
"I think we previously discussed..."
"If I recall correctly..."  
"This feels related to something earlier..."
"I'm not certain whether..."
"Let me think about what I know about..."
```

`STORE` patterns — conclusion formation, insight crystallization, strong belief statements:
```
"So the key point here is..."
"This is an important principle:"
"I'm now confident that..."
"The fundamental insight is..."
"This should be remembered:"
```

`PASS` — pure calculation, logical deduction steps, formatting decisions. No memory signal.

The classifier outputs a `hint` string alongside the signal type. This is a compressed natural language description of what to query or store — the classifier summarizes the relevant window into ~15 words that become the semantic query sent to HelixDB. For example, window containing "thinking about whether the AI should own its infrastructure or rely on third-party managed services" → hint: `"AI infrastructure ownership vs managed services tradeoff"`.

---

## The Store Queue

Storing is always async, always fire-and-forget. Neither the explicit `<remember>` path nor the implicit `STORE` signal ever pauses the thinking stream.

When a store fires, the runtime extracts the content to remember, constructs an Engram with preliminary scores (importance is estimated from signal confidence × emotional language markers × novelty check against existing engrams), and writes it to HelixDB. The write happens in ~3ms. The thinking stream never knows this happened.

The importance scoring at write time is intentionally rough — the consolidation engine (which you said you've already solved) will refine it later. The store queue's job is just to capture the signal before it scrolls out of the buffer window. Speed over precision at write time, precision over speed at consolidation time.

---

## What the Enriched Output Looks Like

A raw thinking block before CTP:

```
I need to think about what infrastructure approach makes sense here.
There's a question of whether to build on managed cloud services or own
the stack. I believe this is something we've discussed before in terms of
sovereignty... [continues reasoning without the memory]
```

The same block after CTP intercepts the implicit `RECALL` signal and injects:

```
I need to think about what infrastructure approach makes sense here.
There's a question of whether to build on managed cloud services or own
the stack. I believe this is something we've discussed before in terms of
sovereignty...

<memory_activation resonance="0.89" source="implicit">
  AI sovereignty discussion [high importance]: User frames infrastructure
  as political, not just technical. Core belief: owning the stack is
  non-negotiable for strategic domains. Emotional charge: 0.74 positive.

  HelixDB architecture session: User explicitly chose Rust + owned DB
  over managed cloud. Repeated theme across 3 sessions.
</memory_activation>

...which confirms the approach — we should build on owned infrastructure.
The managed cloud option is inconsistent with the stated sovereignty principle.
```

The LLM's reasoning pivoted based on memory it didn't explicitly ask for. That's the system working correctly.

---

## The One Thing That Makes This Hard to Build

The injection timing for the async path. You can't inject mid-sentence — it breaks the grammar of the thinking stream and confuses the LLM when it reads back its own thought. You need a real boundary detector that understands when a semantic unit has closed.

A naive implementation uses `.` + `\n` as boundaries. That works 80% of the time. A better implementation adds a small trained boundary detector that understands paragraph-level thought units — the difference between "end of a calculation step" and "end of a reasoning segment." The async injection should always happen at the end of a reasoning segment, not just a sentence.

That boundary detector is the hardest 20% of this runtime to build well — worth calling it out now so it doesn't surprise you later.
