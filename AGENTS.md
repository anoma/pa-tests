# AGENTS

Purpose: fast orientation for agents in this repo.

## Non-Negotiables

- Modify only code explicitly requested by the user.
- Do not perform adjacent cleanup/refactors/renames unless requested.
- If work requires deviating from established patterns, ask the user first.
- Surface out-of-scope issues as recommendations, not edits.
- In Ralph Mode only, proceed autonomously within the user-approved objective and plan.

## Load By Task

- Coding quality rules, naming, control flow, tests: `docs/agents/coding-guidelines.md`
- Crate responsibilities and data flow: `docs/agents/architecture.md`
- State keys/getters/setters patterns: `docs/agents/patterns-state.md`
- Deploy/init and `deploy_and_insert_*` conventions: `docs/agents/patterns-deploy.md`
- Environment/prover/execute/test flow: `docs/agents/patterns-env-tests.md`

## Default Workflow

- Read the smallest relevant doc(s) above.
- Keep edits minimal and within requested scope.
- Run focused tests for touched logic.
- Report changes, verification, and optional follow-ups.
