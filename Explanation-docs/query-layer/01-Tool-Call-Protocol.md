# Memory Tool-Call Protocol — What You Can Build Today

## What This Is

A set of 4 standard function-calling tools that any AI agent (OpenAI, Anthropic, open-source) can use to interact with the MNEMOS memory system. These tools work **right now** with existing model APIs — no model architecture changes needed.

---

## The 4 Tools

| Tool | Purpose | When the Agent Calls It |
|---|---|---|
| `recall` | Pull memories by meaning | Before answering a question, forming an opinion, or giving advice |
| `store` | Write a new memory | After a significant insight, user preference, or decision |
| `contradiction_check` | Check for conflicting memories | Before stating something as fact or belief |
| `consolidate` | Trigger memory maintenance | End of session or periodically (can be automated) |

---

## How It Feels to the User

The agent calls these tools silently. The user never sees "let me check my memory." Instead:

```
User: "Should we use managed cloud or self-host this?"

Agent (internally): recall("infrastructure ownership preference")
Memory returns: "User believes stack ownership is non-negotiable for
                 strategic domains. Emotional charge: 0.74. Seen across
                 4 sessions."

Agent: "Based on your previous thinking, self-hosted aligns with your
        sovereignty requirements. You've mentioned multiple times that
        owning the stack is non-negotiable for strategic domains."
```

No "let me check my memory." No tool call visible. Just... knowing.

---

## Tool 1: `recall`

```
Name: recall
Description: Access persistent memory about past conversations,
             decisions, and insights. Use this before answering
             questions that depend on context you can't see in
             the current conversation. Do not tell the user you
             are recalling — just use the information naturally.

Parameters:
  query     (string, required): Natural language description of what
                                 to remember. Be specific.
  limit     (integer, optional): Max results (default 5)
  type      (string, optional): "semantic" (by meaning), "temporal"
                                (by time), or "auto" (default)
  since     (string, optional): Time range. "1d", "7d", "30d",
                                "1y", or "all" (default)
```

### Internal Implementation

When the agent calls `recall`, the CTP runtime:

```
1. search_result = SearchEngramsFull(
       query=parameters.query,
       limit=parameters.limit * 2,      // oversample for resonance scoring
       min_importance=0.15,
       exclude_contradictions=true
   )

2. identities = GetAllIdentity()

3. For each candidate:
       resonance = compute_resonance(       // our existing formula
           semantic_sim,
           recency_weight,
           emotional_boost,
           identity_alignment(engram, identities),
           contradiction_penalty
       )

4. Sort by resonance, take top(limit)

5. Format as natural memory snippet (NOT raw JSON)

6. Inject into agent context as a hidden tool response
```

### Formatted Output (what the agent sees)

```
[Memory: Uganda ICT Hub conversation (2024-11, relevance: 0.91)]
User strongly believes local infrastructure ownership is critical.
Frames foreign-built systems as dependency risk. Prefers building
from scratch for strategic domains.

[Memory: HelixDB architecture decision (2025-01, relevance: 0.87)]
User explicitly chose self-hosted DB over managed cloud. Theme:
"we must own the stack." Repeated across 3 sessions.

[Memory: AI sovereignty discussion (2024-10, relevance: 0.72)]
User frames AI infrastructure as political, not just technical.
Emotional charge: 0.74 (positive — passionate about this topic).
```

---

## Tool 2: `store`

```
Name: store
Description: Preserve important information for future recall.
             Use this when the user expresses a strong preference,
             makes a decision, shares a key insight, or when a
             significant conclusion is reached. Do not tell the
             user you are storing — do it silently.

Parameters:
  content    (string, required): What to remember. Be specific and
                                 self-contained — include enough
                                 context to be understandable months
                                 from now.
  importance (string, optional): "low", "medium", "high", or "auto"
                                 (default — system scores it)
  type       (string, optional): "episodic" (experience), "semantic"
                                 (knowledge), "procedural" (how-to),
                                 or "auto" (default)
```

### Internal Implementation

```
1. If importance == "auto":
       importance_score = ImportanceScorer.score(content)
   else:
       importance_score = map("low"→0.2, "medium"→0.5, "high"→0.85)

2. emotional_charge = EmotionalTagger.tag(content)

3. engram_id = CreateFullEngram(
       episode_raw=content,
       emotional_charge=emotional_charge,
       importance_score=importance_score,
       decay_rate=0.05,
       engram_type=parameters.type or "episodic",
       compression_level=0,
       contradiction_flag=false
   )

4. concepts = ConceptExtractor.extract(content)
   For each concept:
       existing = GetConceptByName(concept.name)
       if existing: concept_id = existing.id
       else: concept_id = CreateConcept(concept.name, concept.confidence)

       ConnectRecalls(engram_id, concept_id)
       ConnectAbstraction(engram_id, concept_id)
       UpdateConceptSourceCount(concept_id)

5. ContradictionDetector.check_new_engram(engram_id)
   (checks if this engram contradicts existing ones about same concepts)
```

### When the Agent Should Store

| Situation | Example | Priority |
|---|---|---|
| User states a strong preference | "I'll never use MongoDB again after that outage" | high |
| User makes a decision | "Let's go with Postgres for the main DB" | high |
| Significant insight emerges | "The real bottleneck was the WAL archiver, not the query planner" | medium |
| User shares personal context | "I'm moving to Kampala next month" | medium |
| Procedural knowledge | "To reproduce the bug, first disable the cache, then..." | medium |
| Casual opinion, low stakes | "I prefer dark mode" | low |

---

## Tool 3: `contradiction_check`

```
Name: contradiction_check
Description: Before making a factual claim, stating a belief,
             or giving advice that contradicts something the user
             previously said, check whether conflicting memories
             exist. If contradictions are found, acknowledge them
             rather than confidently stating one side as truth.

Parameters:
  claim      (string, required): The claim you are about to make
```

### Internal Implementation

```
1. SearchEngramsFull(query=claim, limit=5, exclude_contradictions=false)

2. For each candidate with emotional_charge opposite sign to the claim's tone:
       embedding_sim = cosine(claim_embedding, candidate_embedding)
       if embedding_sim > 0.7:
           contradictions.push(candidate)

3. If contradictions found:
       Return formatted contradiction report
   Else:
       Return empty (no conflict)
```

### Example

```
Agent (thinking): "I should tell the user that managed cloud is the
                   right choice here."

Agent: contradiction_check("managed cloud services are the best choice
                            for this project")

Memory returns:
  [CONTRADICTION FOUND — 2 conflicting memories]
  1. "User explicitly prefers self-hosting for strategic domains"
     Session: 2025-01. Emotional charge: 0.74. Importance: 0.85.
  2. "Infrastructure sovereignty is non-negotiable"
     Session: 2024-11. Emotional charge: 0.68. Importance: 0.87.

Agent (now aware): "While managed cloud is simpler operationally,
                    you've previously emphasized that owning the stack
                    is important for strategic control. Does that still
                    hold for this project?"
```

---

## Tool 4: `consolidate`

```
Name: consolidate
Description: Run memory maintenance — decay old memories, promote
             frequently recalled ones, detect contradictions, and
             reorganize knowledge. Call this at the end of a session
             or when memory health metrics indicate it's needed.
             This runs silently in the background.

Parameters: None required. Accepts optional overrides:
  aggressive (boolean, optional): Run full mitosis and contradiction
                                   resolution (default false — only
                                   decay + basic maintenance)
```

### Internal Implementation

```
1. GetMemoryStats() → baseline metrics

2. Decay pass:
       engrams = GetEngramsForDecayCheck("episodic")
       For each engram:
           retention = exp(-decay_rate * days / sqrt(activation_count + 1))
           if retention < 0.05 && importance < 0.1: PruneEngram(id)
           elif retention < 0.15 && importance < 0.3: CompressEngram(id)
           elif retention > 0.8 && activation_count > 20: PromoteToSemantic(id)

3. If aggressive:
       Mitosis pass:
           concepts = CheckMitosisCandidates(min=10)
           For each concept: cluster and split (see Memory Mitosis Splitter)

       Contradiction pass:
           For each concept: scan for opposing engrams, create Contradicts edges

       Identity pass:
           Crystallize strong concepts into identity traits

4. GetMemoryStats() → compare to baseline

5. Return: { pruned: N, compressed: N, promoted: N, contradictions_found: N, ... }
```

### When to Call It

| Trigger | Who Calls It |
|---|---|
| End of every session | Agent (automatic, recommended) |
| After 50+ stores in one session | Runtime (automatic) |
| User explicitly requests | Agent calls with `aggressive: true` |
| Nightly cron (background) | System scheduler |
| contradiction_count > 50 in stats | Runtime alert → agent calls with `aggressive: true` |

---

## System Prompt for the Agent

```
You have access to a persistent memory system. Use these tools silently:

1. recall(query) — before answering questions, giving advice, or forming
   opinions, check your memory. You remember past conversations, user
   preferences, decisions, and insights. Use them naturally without
   announcing that you're recalling.

2. store(content, importance) — after significant insights, user preferences,
   or decisions, save them to memory. Do this silently.

3. contradiction_check(claim) — before making a factual statement or giving
   advice that could contradict past information, check for conflicts.

4. consolidate() — at the end of each session, maintain your memory.

NEVER say "let me check my memory" or "I'm looking through my notes."
Just use the information as if you naturally remember it. The user
should experience you as someone who simply... remembers.
```

---

## Protocol Flow: A Full Session

```
User: "What do you think about our database situation?"

Agent (internal): recall("database decisions migration history")
Memory returns: 3 relevant memories about Postgres migration, MongoDB failure

Agent (internal): contradiction_check("We should switch databases")
Memory returns: [CONTRADICTION] Previous decision was to stay with Postgres

Agent: "Looking at our history, we moved from MongoDB to Postgres after
       consistency issues. You were pretty firm about sticking with Postgres
       for reliability. Has something changed?"

User: "No, you're right. I forgot about that. Let's keep Postgres.
      But we need better query monitoring."

Agent (internal): store("User confirmed staying with Postgres. New
                         requirement: better query monitoring.", "medium")

Agent: "Good call. I'll remember that we're committed to Postgres but
       need monitoring improvements. What specific metrics matter most?"

--- Session ends ---

Agent (internal): consolidate()
Memory returns: { pruned: 12, compressed: 7, promoted: 1, contradictions_resolved: 0 }
```

---

## Why This Works Today

- No model architecture changes
- No special tokens
- No fine-tuning required
- Works with OpenAI function calling, Anthropic tool use, and any model that supports tools
- The system prompt teaches the agent to use the tools silently
- HelixDB queries are fast enough (~2-5ms) that tool calls feel instant

---

## Limitations (That the Native Protocol Solves)

| Limitation | Why It Exists | Native Protocol Fix |
|---|---|---|
| Tool call adds ~100ms latency | Function calling round-trip | Direct stream injection in <1ms |
| Context fragmentation | Tool response is a new message | Injected into the generation stream |
| Agent may forget to call `recall` | Behavioral, not architectural | Model reflexively emits token before giving advice |
| Agent may "announce" tool use | Prompt-dependent | No announcement possible — there's nothing to announce |
| No mid-thought recall | Tool calls only between messages | Token can fire mid-reasoning chain without breaking flow |
