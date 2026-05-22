# Environment and Tests Patterns

Scope: integration environment setup, proving, execution, and test style.

## Environment Structure

- Integration env lives under `crates/harness/evm/src/envs/integration_test/`.
- E2e env lives under `crates/harness/evm/src/envs/e2e/` (feature = "e2e", remote queue proving).
- This is the current EVM location; other backends may differ.
- Keep setup concerns split by file (`setup`, `prover`, `evm_execute`, `evm_convert`).
- Environment fields are public for setup-time mutation/inspection.
- Runtime APIs should stay trait-based via `pa-test-harness-core::environment`.

## Setup and Execution

- Use `Environment::setup_bare()` for minimal baseline.
- Use `Environment::setup(async |builder| ...)` for extensions.
- Insert all setup outputs via typed state helpers.
- Execute via core helpers in tests:
  - `prove_actions(&env, ...)`
  - `execute_tx(&mut env, tx)`
  - `commitment_root(&env)`

## Testing Style

- Prefer generic tests over `impl Environment` when possible.
- Keep negative tests explicit:
  - proving failures use panic/error assertions,
  - execution failures assert revert details.
- For tamper tests, mutate real proof/seal bytes and return `anyhow::Result<()>` with context.
- Keep assertions tight but stable.

## Do / Do Not

- Do keep helper utilities in test crate `src/lib.rs` when reused by multiple tests.
- Do run focused crate tests after edits.
- Do not couple test logic to concrete env internals unless setup-only.
