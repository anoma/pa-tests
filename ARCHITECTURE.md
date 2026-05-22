# Architecture

This workspace is organized around a backend-agnostic testing core with backend-specific harness implementations.

## Design goals

- Support both local integration tests and end-to-end tests against real deployments.
- Keep test execution generic across backends.
- Isolate backend-specific setup, deploy, and state concerns.
- Reuse shared witness and execution helpers across test crates.
- Keep extension crates optional and composable.

## Crate map

### `crates/harness/core`

Defines the shared contract used by all backends:

- `Environment`
- `Prover`
- `ProtocolAdapter`
- `Transaction`
- `CommitmentTree`
- generic helpers: `prove_actions`, `execute_tx`, `commitment_root`
- typed state container: `State` / `StateBuilder`
- witness types under `witness`

This crate is backend-agnostic.

The trait set provides dependency injection points for proving, execution, and state access, so test logic can remain generic while setup targets different backends or deployment modes.

### `crates/harness/evm`

EVM implementation of the core traits.

Main areas:

- `envs/integration_test/`
  - `setup.rs` - local chain bootstrapping and environment assembly
  - `prover.rs` - witness-to-transaction proving path
  - `evm_execute.rs` - preflight, on-chain execution, and revert diagnostics
  - `evm_convert.rs` - transaction conversion glue
- `envs/e2e/` - remote-queue proving environment gated behind `feature = "e2e"`. Setup forks Sepolia via Anvil, reads verifier params from a reference PA, deploys a fresh PA, and constructs a `QueueClient` for the remote GPU proving queue. Proving submits base logic/compliance jobs concurrently via `try_join_all`, assembles an aggregation proof payload, and polls for the final aggregated transaction.
- `state/` - EVM state keys and typed getters/setters
- `pa.rs` - protocol adapter bindings and deployment helpers
- `mock_risc0_bindings.rs` - mock verifier stack deployment

### `crates/harness/evm-erc20`

ERC-20 utilities for EVM scenarios:

- token contract bindings
- deploy/mint helpers
- typed state insertion and retrieval for token addresses

### `crates/harness/evm-erc20-forwarder`

ERC20 forwarder utilities for EVM transfer scenarios:

- forwarder contract bindings
- deploy and deploy+insert helpers
- typed state insertion and retrieval for forwarder addresses

### `crates/harness/evm-mock-permit2`

Utility for deploying and validating Permit2 at canonical address in local test environments.

### `crates/harness/evm-action-trivial`

Reusable builders for trivial action witness sets, including controlled invalid variants for negative tests.

### `crates/harness/evm-action-transfer`

Reusable builders for transfer witness action sets:

- wrap, transfer, and unwrap action builders
- Permit2 signing helpers and deterministic fixtures
- override-driven invalid variants for negative tests

### `crates/tests/evm`

Integration tests that exercise harness behavior through core abstractions and the EVM backend implementation.

## Data flow

1. Test setup constructs an environment and populates backend state.
2. Tests build action witnesses (e.g., via `evm-action-trivial` or `evm-action-transfer`).
3. `prove_actions` delegates to backend prover and returns backend transaction type.
4. `execute_tx` delegates to backend protocol adapter execution.
5. Successful execution updates the commitment tree; tests assert roots and error paths.

Setup can target local ephemeral deployments for integration testing or real deployed contracts for end-to-end testing. For the e2e env, proving submits jobs to a remote queue instead of running the prover locally.

## State and namespacing

State keys are backend-scoped (for example `evm.*`, future `solana.*`).
Core abstractions remain backend-agnostic and should not assume a specific namespace.

## Extension model

Optional backend features and extension crates are used for deploy and bindings behavior.
Base environments stay minimal; additional dependencies are layered in only when needed by a test scenario.
