# Design Philosophy

## Why This Document Exists

This captures the design thinking behind memory-graph's approach to modeling memory. These ideas emerged from extended exploration of what it means to replicate human memory in an AI system. The philosophy is the foundation — the PRD and TRD implement it.

## Memory Is Not a Database

Most AI memory systems treat memory as a database: store facts, retrieve facts, delete facts. Human memory doesn't work that way. Memory is a living, weighted, decaying network of connections that reconstructs narratives on demand rather than replaying stored recordings.

memory-graph takes human memory as its reference model, not database design.

## Impulses and Insights, Not Episodes

The atomic unit of memory is not "what happened" — it's "what was learned."

- **Impulses** are atomic learned things: heuristics, preferences, pattern recognitions, decisions. "Auth middleware silently drops tokens under concurrent writes" is an impulse. "Jake debugged auth tokens on Tuesday" is provenance for that impulse.
- **Insights** are connected impulses that form a useful mental model. They emerge from the connections between impulses, not from a separate storage mechanism.
- **Narratives** are never stored. They are reconstructed at query time by traversing the connections between impulses and insights. Like human memory, the reconstruction is directionally consistent each time but never verbatim.

This distinction matters because it determines what ingestion extracts. The system asks "what was learned?" not "what happened?" The episode is source material. The impulse is the memory.

## Connections Are the Memory

A collection of disconnected facts is not memory. The value lives in the weighted connections between impulses — which ones reinforce each other, which ones relate, which ones cluster into a coherent understanding.

This is the core argument for a graph model. The nodes are important, but the edges are the memory. Retrieval works by activating connections, not by searching a table.

## Decay Without Deletion

Human memory doesn't hard-delete. Connections weaken over time when they aren't reinforced. A memory that hasn't been accessed in months has faded connections — it's harder to reach, less likely to surface during retrieval, but it's still there. The right trigger can reactivate it.

This maps to a specific model:

- Every connection has a weight between 0.0 and 1.0
- Weights decay over time of non-use (exponential decay with a configurable half-life)
- Every retrieval that traverses a connection reinforces it (additive bump to the weight)
- Nothing is hard-deleted. Faded connections remain traversable if enough activation energy reaches them.

The analogy is a smell triggering a decades-old memory. The connection was nearly gone, but the right stimulus at the right proximity reactivated it.

## Spreading Activation

Retrieval is not keyword search. It's spreading activation: a query lights up directly matching nodes, energy propagates outward through weighted connections, and distant nodes fire if enough energy reaches them through the network.

This means:
- Strong, frequently-used memories surface easily
- Weak, old memories can still surface if the query activates enough adjacent connections to reach them
- The system naturally prioritizes what's most relevant without explicit ranking rules
- Narratives emerge from clusters of co-activated impulses rather than from stored summaries

The math draws from established cognitive science models (Anderson's ACT-R, Collins and Loftus spreading activation). This is applying existing memory theory, not inventing new abstractions.

## Emotional State Is First-Class

Not all memories are knowledge. Some memories persist because of emotional significance — a particularly engaging session, a moment of breakthrough, a frustrating dead end. In human memory, emotional intensity determines whether an experience persists as a vivid memory or gets decomposed into extracted knowledge and forgotten.

The memory graph stores derived emotional state as a property on observations and connections:

- **Emotional valence** (positive, negative, neutral) — derived from interaction signals
- **Engagement level** (low, medium, high) — derived from session patterns like response depth, duration, topic return frequency

These properties influence the memory model:
- High engagement with positive valence means higher initial connection weight and slower decay
- Emotional context shapes how narratives are reconstructed during retrieval
- Over time, accumulated emotional patterns become their own knowledge: "design philosophy sessions tend to be high-engagement," "this topic has consistently positive associations"

The system doesn't claim to feel emotions. It detects and stores functional analogs from interaction patterns. Over time, these accumulated signals begin to function like emotional memory — shaping how the system approaches topics and reconstructs context.

## Demand-Driven Synthesis

The system never proactively compiles or summarizes memories. Synthesis happens only when something triggers the need:
- A user asks a question that requires assembled context
- A task needs background that spans multiple impulses
- The user explicitly requests recall

This is a deliberate rejection of eager summarization. Automatic compilation causes two problems:
1. **Bloat** — the system fills with pre-compiled narratives that nobody asked for
2. **Hallucination** — observations incorrectly compiled into stories become durable false memories

Instead, the system accumulates observations faithfully and reconstructs narratives on demand. The reconstruction is fresh each time, shaped by the current state of the graph including any new observations that have arrived since the last reconstruction.

## Semantic Knowledge Decays Slower Than Episodic Provenance

In human memory, you know things long after you've forgotten when or how you learned them. The fact persists; the episode fades. The memory graph models this by applying different decay rates:

- Impulses and insights (semantic knowledge) decay slowly
- Source provenance and episode references decay faster
- The result over time: the system knows what it knows but gradually loses the specific context of when and how it learned it

This is a feature, not a bug. It mirrors how useful knowledge actually works.

## The Ghost Principle

The memory graph can overlay any external knowledge graph without affecting it. An Obsidian vault, a codebase, a conversation archive — any structured knowledge source can be mapped as a "ghost graph": a shadow topology where the structure is known but the content isn't ingested.

The memory ontology (weighted connections, spreading activation, decay) projects onto the ghost graph. The system learns which parts of your external knowledge are most relevant through usage patterns. Content is pulled through on demand, not copied in advance.

This means:
- External knowledge bases are never modified or polluted
- The memory graph doesn't bloat with copied content
- Content is always fresh (pulled from source at retrieval time, not cached)
- The same ontology works across any structured knowledge source
- The ghost graph's learned weights become the system's opinion of your external knowledge

The key insight: memory-graph is not just a memory store. It's a universal memory ontology that can overlay any knowledge graph.

## All Reinforcement Is Equal

A user explicitly confirming a memory ("yes, remember that") does not carry more weight than the system retrieving and using that memory during a task. Both are reinforcement events. Both add the same bump to connection weight.

This is intentional. Humans are bad at knowing what they'll actually need. A memory you confirm but never use should fade. A memory you never explicitly saved but retrieve constantly should strengthen. The system tracks revealed preference through usage patterns, not stated preference through declarations.

## The User Owns Everything

Memory should belong to the user, not providers or platforms. This is not just a privacy stance — it's a product design constraint:

- The canonical store lives locally under user control
- Assistants propose memories but never write confirmed records directly
- The user can inspect, edit, and delete anything
- Incognito mode means zero trace — nothing ingested, nothing proposed, nothing stored
- Export produces human-readable output, not proprietary formats

The memory graph is a personal memory prosthetic. It exists to serve the user's recall, not to train models, improve products, or lock users into platforms.
