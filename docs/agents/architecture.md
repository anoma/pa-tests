# Architecture

Scope: crate responsibilities and data flow.

## Workspace Layout

- `crates/harness/core`: generic traits, state store, witness types, helpers (`prove_actions`, `execute_tx`, `commitment_root`).
- `crates/harness/evm`: EVM harness implementation, state helpers, PA/mock Risc0 deploy and integration env.
- `crates/harness/evm-erc20`: ERC-20 bindings and token address state helpers.
- `crates/harness/evm-erc20-forwarder`: ERC20 forwarder bindings, deploy helpers, and forwarder address state helpers.
- `crates/harness/evm-mock-permit2`: optional Permit2 canonical deployment helper.
- `crates/harness/evm-action-trivial`: trivial action witness builders for test scenarios.
- `crates/harness/evm-action-transfer`: transfer witness action builders for wrap/transfer/unwrap and negative test variants.
- `crates/tests/evm`: cross-crate EVM tests using harness abstractions.

## Core Flow

- Setup builds concrete env and populates typed `State` keys.
- Tests construct witnesses (often via `evm-action-trivial` or `evm-action-transfer`).
- `prove_actions` delegates to env prover.
- `execute_tx` delegates to protocol adapter execution.
- Successful execution updates commitment tree; tests assert roots and failures.

## Design Boundaries

- Setup code may touch concrete env fields directly.
- Test execution code should stay generic over `impl Environment`.
- State key namespaces are backend-scoped (`evm.*`, `solana.*`, etc.); keep core patterns backend-agnostic.
- Keep state access via typed state helper modules, not ad hoc string keys in tests.
- Keep feature-gated modules aligned with crate features.
