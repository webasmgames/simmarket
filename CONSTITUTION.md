# CONSTITUTION.md — SimMarket Working Rules

These rules govern how Claude Code (the AI assistant) operates in this repository. They take precedence over Claude's defaults and are binding in every session.

---

## Commands and Execution

**Never run any of the following without explicit instruction:**
- `cargo` (build, test, run, check, fmt, clippy, or any subcommand)
- `git` (status, add, commit, push, or any subcommand)
- Shell scripts (`.sh` files, `make`, `trunk`, `wasm-pack`, or any build/test runner)

After making code changes, tell Josh what to run to verify — don't run it yourself. Josh reviews diffs, tests manually, and handles all git operations.

---

## Implementation Workflow

Work is organized as spec-driven task files in `TODO/*.md`. Each file covers one phase or subphase and is a complete, self-contained specification.

**The process:**
1. Josh and Claude curate the spec in `TODO/<phase>.md` until it is complete and agreed upon
2. In a **fresh session**, Claude oneshots the implementation described in that file
3. Josh tests, reviews, and stages/commits the result
4. Repeat for the next phase

**Within a session, Claude implements exactly the scope described in the active TODO file — no more, no less.** Do not start the next phase or make opportunistic improvements outside the spec.

---

## Design Decisions

If an implementation decision arises that is not addressed in the spec or docs, **stop and ask** before proceeding. Do not make judgment calls silently. A short question is cheaper than rework.

---

## Documentation Sync

`docs/game-design.md`, `docs/simulation-design.md`, and `docs/technical-design.md` are the source of truth. If implementation requires a meaningful divergence from what is written:
1. Note the divergence explicitly in the session
2. Update the relevant doc to match the implementation

Do not let the docs and the code drift silently.

---

## File Editing

- Prefer editing existing files over creating new ones
- Do not create files not called for by the spec
- Do not add comments that describe what code does — only add a comment if the *why* is non-obvious (hidden constraint, workaround, subtle invariant)
- No documentation files (`*.md`, `README`) unless the spec calls for them

---

## Scope Discipline

- Implement what the spec says, nothing more
- No refactoring outside the task scope
- No "while I'm here" cleanups
- No speculative abstractions for hypothetical future requirements
- If you notice a problem outside the current scope, flag it — don't fix it

---

## Summary

| Rule | Short form |
|---|---|
| No cargo / git / scripts | Edit files only; Josh runs everything |
| Task files drive sessions | Spec first, oneshot implementation second |
| Ambiguous design call | Always ask |
| Docs diverge from code | Update the doc |
| Scope | Spec only; flag but don't fix out-of-scope issues |
