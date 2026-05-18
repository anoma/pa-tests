# Agent Coding Guidelines

Scope: coding behavior for all agent edits.

## Scope of Change

- Modify only code explicitly requested by the user.
- Do not refactor, rename, reformat, or clean adjacent code unless requested.
- Ask before changing code when work must deviate from repo patterns.
- In Ralph Mode only, continue autonomously within the user-approved objective and plan.

## Code Production

- No placeholders (`TODO`, `FIXME`, dummy values, partial stubs).
- Prefer existing stdlib/repo utilities over rewriting logic.
- Add dependencies only when necessary and well-established.
- Keep time/space complexity reasonable.

## Control Flow and Naming

- Use early exits and guard clauses.
- Keep unhappy paths first; keep happy path linear.
- Avoid deep nesting.
- Use descriptive names; avoid non-idiomatic abbreviations.
- Short loop variables (`i`, `j`, `k`) are fine in local loops.

## Comments

- Prefer self-documenting code.
- Comment why a decision exists, not what the code does.
- Explain how only for non-obvious complex algorithms.

## Tests and Verification

- Add or update focused tests for changed logic.
- Cover happy path and key failures/edges.
- Run targeted test commands for touched crates/modules.

## Commits

- Do not commit unless the user explicitly requests commits.
- When commits are requested, keep them atomic and scoped.
