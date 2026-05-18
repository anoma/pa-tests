# State Patterns

Scope: define and access harness state.

## Conventions

- Keep key constants in `state/keys.rs` or module-local constants when specific.
- Use backend-scoped key namespaces (for example, `evm.*`, `solana.*`).
- Keep `core` state conventions backend-agnostic.
- Expose typed helpers in `state/*` modules:
  - `insert_*(&mut StateBuilder, ...)`
  - `*_in_state(&State, ...) -> anyhow::Result<T>`
  - `*<E: Environment>(&E, ...) -> anyhow::Result<T>`
- Add context-rich errors.

## Existing Examples

- `crates/harness/evm/src/state/pa.rs`: `KEY_PA_ADDRESS = "evm.pa.address"`.
- `crates/harness/evm/src/state/actors.rs`: `KEY_DEFAULT_SIGNER = "evm.actor.default_signer"`.
- `crates/harness/evm/src/state/keys.rs`: chain key builder with `NamedChain` slug.
- `crates/harness/evm-erc20/src/state/keys.rs`: `evm.erc20.addr.<symbol>`.

## Do / Do Not

- Do centralize key formats behind helper functions.
- Do test key format and insert/get behavior.
- Do not hardcode key strings across unrelated modules.
- Do not bypass typed helpers from test logic when helpers already exist.
