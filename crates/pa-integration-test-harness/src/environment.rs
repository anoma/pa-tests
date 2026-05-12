mod simulated;

use std::borrow::Cow;

use alloy::primitives::Address;
use alloy::providers::Network as AlloyNet;
use alloy::providers::Provider as AlloyProvider;
use anoma_rm_risc0::transaction::Transaction;

use crate::witness::ActionWitnesses;

pub trait Environment {
    /// Network of the environment.
    type Network: AlloyNet;

    /// [Alloy provider](AlloyProvider) of the environment.
    type Provider: AlloyProvider<Self::Network>;

    /// ARM transaction.
    type Transaction: Into<Transaction>;

    /// The [alloy provider](AlloyProvider) of this environment.
    fn provider(&self) -> &Self::Provider;

    /// Deployed instance of the protocol adapter.
    fn pa_address(&self) -> Cow<'_, Address>;

    /// Create an ARM transaction.
    ///
    /// This will attempt to prove the transaction, so invalid witnesses
    /// will result in an error.
    fn create_txn(&self, actions: Vec<ActionWitnesses>) -> anyhow::Result<Self::Transaction>;
}
