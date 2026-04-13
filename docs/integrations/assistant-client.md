# Assistant Client Note

## Summary

An assistant is treated here as an early client of the portable memory graph. It can act as an injection surface for new memory and a retrieval surface for relevant context, but it should not own the canonical memory model.

## Responsibilities

An assistant client should be able to:

- send interaction events or explicit memory saves into the memory graph
- request relevant context before chat, planning, or coding actions
- render recalled context inside its own task flow
- expose provenance or recall explanations when useful
- support explicit user actions such as "remember this" and "why was this recalled?"

## Non-Goals

The assistant client should not:

- define the canonical graph schema
- become the system of record for portable memory
- own sync behavior
- force the memory graph into an assistant-specific format
- assume that all memory lives inside assistant-managed notes

## Integration Boundary

The integration should remain thin.

- The assistant contributes events, candidate memories, and retrieval requests.
- The memory graph handles persistence, provenance, and memory lifecycle semantics.
- Existing assistant-local memory systems can remain as fallback or compatibility layers if needed.

## Failure And Fallback

The client should degrade gracefully when the memory graph is unavailable.

- retrieval failures should not block the core assistant workflow
- writes can be skipped, queued, or surfaced to the user instead of failing silently
- assistant-local notes or lightweight memory can remain available as a temporary fallback

## Open Questions

- how much memory should be auto-saved versus explicitly confirmed
- where retrieval should be injected into planning and coding flows
- how much provenance should be shown by default in the user experience
- what local transport should be used once the broader service contract becomes clearer
