# TODO: [Phase Name] — [Short Description]

## Overview

> One paragraph. What does this phase accomplish and why does it exist at this point in the build? What does the system look like before this phase, and what does it look like after?

---

## Requirements

> What must be true when this phase is complete? Written as acceptance criteria — verifiable statements, not implementation notes. Each item should be testable by Josh manually or via `cargo test`.

- [ ] Requirement 1
- [ ] Requirement 2
- [ ] Requirement 3

---

## Design

> Technical approach for this phase. Reference `docs/technical-design.md` or `docs/simulation-design.md` as needed, but spell out anything specific to this phase. Include:
> - Data structures introduced or modified
> - Key algorithms or logic
> - How this phase connects to what came before and what comes after
> - Any deviations from the main design docs (flag these explicitly)

### Data Structures

```rust
// Relevant types, sketched out
```

### Key Logic

> Pseudocode or prose description of the non-obvious parts.

---

## Files

> Exhaustive list of files to create or modify. Claude should not touch files not listed here.

| File | Action | Notes |
|---|---|---|
| `src/sim/exchange.rs` | Create | LOB implementation |
| `src/shared/types.rs` | Modify | Add OrderId, AgentId |

---

## Tasks

> Discrete, ordered implementation steps. These map directly to what Claude does in the oneshot session. Each task should be small enough to be verifiable on its own.

- [ ] **1.** [Task description — specific enough that there's one right answer]
- [ ] **2.** [Task description]
- [ ] **3.** [Task description]

---

## Out of Scope

> Explicitly list anything that is *not* part of this phase, especially things that might seem like natural extensions. This prevents scope creep during the oneshot session.

- Item X will be handled in Phase N
- No UI for this phase — headless only
- No error handling beyond panics until Phase N

---

## Open Questions

> Design questions that must be resolved before the oneshot session begins. This section should be empty (or deleted) before handing the file to Claude for implementation.

- [ ] Question 1?
- [ ] Question 2?

---

## Manual Testing

> Claude fills this out as part of the spec. Concrete, step-by-step scenarios Josh can run by hand to verify the phase works correctly. Each item should be specific enough to execute without guessing.

- [ ] 
- [ ] 
- [ ] 

---

## Green Light

- [ ] Approved

---

## Notes

> Anything else Claude should know: known gotchas, relevant prior art, links to reference material, constraints from the simulation design doc.
