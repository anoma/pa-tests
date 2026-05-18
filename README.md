# Protocol Adapter Test Harness

A lightweight, multi-backend test harness for Protocol Adapter integration and end-to-end testing.

This repository provides a backend-agnostic harness core, EVM-specific harness and support crates, reusable action builders for generating test inputs, and EVM test suites. The same test logic can run in local integration-style setups or end-to-end flows against real deployments by injecting backend behavior through shared core traits.

For a deeper walkthrough of crate responsibilities and data flow, see [ARCHITECTURE.md](./ARCHITECTURE.md).

## Workspace overview

- `crates/harness/core` - shared traits, state container, witness types, test helpers
- `crates/harness/evm` - EVM environment, setup/prover/execute paths, EVM state helpers
- `crates/harness/evm-erc20` - ERC-20 deploy/binding utilities for tests
- `crates/harness/evm-mock-permit2` - optional Permit2 canonical-address deployment helper
- `crates/harness/evm-action-trivial` - trivial action witness builders
- `crates/tests/evm` - integration tests using the harness

## Quick start

```bash
cargo test -p pa-evm-tests
```

For targeted runs, use crate-local tests or specific test names under `crates/tests/evm/tests/`.
