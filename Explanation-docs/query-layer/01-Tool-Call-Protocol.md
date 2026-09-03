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

```rust
// Step 1: Compute query embedding client-side
let query_embedding: Vec<f32> = embed(&parameters.query);

// Step 2: Vector search on Engram directly (no EngramVector + HasVector traversal)
let search_request = search_engrams_full(
    query_embedding,
    parameters.limit * 2,   // oversample for resonance scoring
    0.15,                   // min_importance
);
let search_response = client.query(search_request).send().await?;
let candidates: Vec<EngramData> = serde_json::from_value(
    search_response["candidates"].clone()
)?;

// Step 3: Fetch identity traits for resonance scoring
let identity_request = get_all_identity();
let identity_response = client.query(identity_request).send().await?;
let identities: Vec<IdentityData> = serde_json::from_value(
    identity_response["traits"].clone()
)?;

// Step 4: Score each candidate with CRR resonance formula
let mut scored: Vec<ScoredEngram> = candidates.iter().map(|engram| {
    let semantic_sim = 1.0 - engram.$distance;  // from vector search
    let recency_weight = compute_recency(&engram.timestamp);
    let emotional_boost = engram.emotional_charge.abs();
    let identity_alignment = compute_alignment(engram, &identities);
    let contradiction_penalty = if engram.contradiction_flag { 0.5 } else { 1.0 };

    let resonance = semantic_sim * recency_weight * (1.0 + emotional_boost)
        * identity_alignment * contradiction_penalty;

    ScoredEngram { engram, resonance }
}).collect();

// Step 5: Sort by resonance, take top(limit)
scored.sort_by(|a, b| b.resonance.partial_cmp(&a.resonance).unwrap());
let top_results: Vec<_> = scored.into_iter().take(parameters.limit as usize).collect();

// Step 6: Format as natural memory snippets (NOT raw JSON)
let formatted = format_memories(&top_results);

// Step 7: Inject into agent context as a hidden tool response
inject_into_context(formatted);
```

**Key simplification:** Vector search uses `g().vector_search_with("Engram", "embedding", ...)` which returns `Engram` nodes directly. No `::In<HasVector>` traversal needed — the embedding lives on the `Engram` node itself.

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

```rust
// Step 1: Score importance
let importance_score = if parameters.importance == "auto" {
    importance_scorer.score(&parameters.content)
} else {
    match parameters.importance.as_str() {
        "low" => 0.2,
        "medium" => 0.5,
        "high" => 0.85,
        _ => 0.5,
    }
};

// Step 2: Tag emotional charge
let emotional_charge = emotional_tagger.tag(&parameters.content);

// Step 3: Compute embedding (client-side)
let embedding: Vec<f32> = embed(&parameters.content);

// Step 4: CreateFullEngram — ONE step, embedding stored directly on Engram node
// No separate EngramVector node, no HasVector edge needed
let create_request = create_full_engram(
    parameters.content.clone(),  // episode_raw
    emotional_charge,
    importance_score,
    0.05,                        // decay_rate
    parameters.type.clone().unwrap_or_else(|| "episodic".to_string()),
    0,                           // compression_level
    false,                       // contradiction_flag
    embedding,                   // stored directly on Engram.embedding
);
let create_response = client.query(create_request).send().await?;
let engram_id: u64 = serde_json::from_value(
    create_response["engram"]["$id"].clone()
)?;

// Step 5: Extract concepts and link
let concepts = concept_extractor.extract(&parameters.content);
for concept in concepts {
    let existing_request = get_concept_by_name(concept.name.clone());
    let existing_response = client.query(existing_request).send().await?;

    let concept_id: u64 = if !existing_response["concept"].is_null() {
        serde_json::from_value(existing_response["concept"]["$id"].clone())?
    } else {
        let create_concept_request = create_concept(
            concept.name,
            concept.confidence,
        );
        let create_concept_response = client.query(create_concept_request).send().await?;
        serde_json::from_value(create_concept_response["concept"]["$id"].clone())?
    };

    let recalls_request = connect_recalls(engram_id, concept_id);
    client.query(recalls_request).send().await?;

    let abstraction_request = connect_abstraction(engram_id, concept_id);
    client.query(abstraction_request).send().await?;

    // Update source count (read current, then set)
    let current_count = get_source_count(concept_id).await?;
    let update_request = update_concept_source_count(concept_id, current_count + 1);
    client.query(update_request).send().await?;
}

// Step 6: Check for contradictions
contradiction_detector.check_new_engram(engram_id).await?;
```

**Key simplification:** `create_full_engram()` creates the `Engram` node with the `embedding` property in a single `add_n("Engram", ...)` call. No separate `EngramVector` node creation, no `HasVector` edge. The old 3-step process (create engram → create vector → connect with edge) is now 1 step.

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

```rust
// Step 1: Compute claim embedding
let claim_embedding: Vec<f32> = embed(&parameters.claim);

// Step 2: Vector search on Engram directly (no traversal needed)
let search_request = search_engrams_full(
    claim_embedding,
    5,       // limit
    0.0,     // min_importance — include all for contradiction check
);
let search_response = client.query(search_request).send().await?;
let candidates: Vec<EngramData> = serde_json::from_value(
    search_response["candidates"].clone()
)?;

// Step 3: Detect contradictions
let claim_tone = emotional_tagger.tag(&parameters.claim);
let mut contradictions: Vec<EngramData> = Vec::new();

for candidate in &candidates {
    // Check for opposite emotional charge (sign flip)
    if candidate.emotional_charge.signum() == -claim_tone.signum() {
        // Embedding similarity — both embeddings on Engram nodes directly
        let embedding_sim = cosine_similarity(
            &claim_embedding,
            &candidate.embedding,  // direct access, no traversal
        );
        if embedding_sim > 0.7 {
            contradictions.push(candidate.clone());
        }
    }
}

// Step 4: Return formatted report or empty
if contradictions.is_empty() {
    return Ok(ContradictionReport::empty());
}

Ok(ContradictionReport {
    found: true,
    conflicts: contradictions,
})
```

**Key simplification:** Both the query embedding and candidate embeddings are accessed directly on `Engram` nodes. No `::In<HasVector>` traversal to reach a separate `EngramVector` node — the embedding is right there on the candidate.

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

```rust
// Step 1: Baseline metrics
let baseline_request = get_memory_stats();
let baseline_response = client.query(baseline_request).send().await?;
let baseline: MemoryStats = serde_json::from_value(baseline_response)?;

// Step 2: Decay pass
let engrams_request = get_engrams_for_decay_check("episodic".to_string());
let engrams_response = client.query(engrams_request).send().await?;
let engrams: Vec<EngramData> = serde_json::from_value(
    engrams_response["engrams"].clone()
)?;

let mut pruned = 0;
let mut compressed = 0;
let mut promoted = 0;

for engram in &engrams {
    let days = days_since(&engram.timestamp);
    let retention = (-engram.decay_rate * days as f64
        / ((engram.activation_count as f64 + 1.0).sqrt())).exp();

    if retention < 0.05 && engram.importance_score < 0.1 {
        // PruneEngram — single drop, no vector cascade needed
        // (embedding is on the Engram node itself, so dropping the
        //  node removes everything — no separate EngramVector to clean up)
        let prune_request = prune_engram(engram.$id);
        client.query(prune_request).send().await?;
        pruned += 1;
    } else if retention < 0.15 && engram.importance_score < 0.3 {
        let compress_request = compress_engram(engram.$id);
        client.query(compress_request).send().await?;
        compressed += 1;
    } else if retention > 0.8 && engram.activation_count > 20 {
        let promote_request = promote_to_semantic(engram.$id);
        client.query(promote_request).send().await?;
        promoted += 1;
    }
}

// Step 3: If aggressive mode
if parameters.aggressive.unwrap_or(false) {
    // Mitosis pass
    let mitosis_request = check_mitosis_candidates(10);
    let mitosis_response = client.query(mitosis_request).send().await?;
    let overloaded_concepts: Vec<ConceptData> = serde_json::from_value(
        mitosis_response["concepts"].clone()
    )?;
    for concept in overloaded_concepts {
        cluster_and_split(concept).await?;
    }

    // Contradiction pass
    let contradicted_request = get_contradicted_engrams();
    let contradicted_response = client.query(contradicted_request).send().await?;
    let contradicted: Vec<EngramData> = serde_json::from_value(
        contradicted_response["engrams"].clone()
    )?;
    for engram in contradicted {
        resolve_contradictions(engram).await?;
    }

    // Identity pass
    crystallize_identity_traits().await?;
}

// Step 4: Compare to baseline
let final_request = get_memory_stats();
let final_response = client.query(final_request).send().await?;
let final_stats: MemoryStats = serde_json::from_value(final_response)?;

// Step 5: Return summary
Ok(ConsolidationResult {
    pruned,
    compressed,
    promoted,
    contradictions_found: final_stats.contradictions - baseline.contradictions,
})
```

**Key simplification:** `prune_engram()` is a single `g().n(NodeRef::id(id)).drop()` — no vector cascade needed. Since the embedding lives directly on the `Engram` node, dropping the node removes everything. The old design required cascading deletes to also remove the separate `EngramVector` node and `HasVector` edge.

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
