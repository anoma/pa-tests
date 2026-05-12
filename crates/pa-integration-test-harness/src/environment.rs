mod simulated;

use std::borrow::Cow;

use alloy::primitives::{Address, U256};
use anoma_rm_risc0::Digest;
use anoma_rm_risc0::action_tree::MerkleTree;
use async_trait::async_trait;

use crate::witness::ActionWitnesses;

/// Environment used to test the protocol adapter.
///
/// When initialized, the environment is assumed to contain the following:
///
/// - The canonical Permit2 deployment.
/// - The Risc0 verification stack (router, emergency stopper, and base verifier)
/// - Some ERC-20 token deployments.
/// - The EVM protocol adapter.
pub trait Environment {
    /// ARM transaction.
    type Transaction;

    /// ERC-20 tokens.
    type Erc20Tokens: Erc20Tokens;

    /// Protocol adapter.
    type ProtocolAdapter: ProtocolAdapter<Transaction = Self::Transaction>;

    /// Transaction prover.
    type Prover: Prover<Transaction = Self::Transaction>;

    /// Get a reference to the ERC-20 tokens state.
    fn erc20_tokens(&self) -> &Self::Erc20Tokens;

    /// Get a mut reference to the ERC-20 tokens state.
    fn erc20_tokens_mut(&mut self) -> &mut Self::Erc20Tokens;

    /// Get a reference to the protocol adapter.
    fn protocol_adapter(&self) -> &Self::ProtocolAdapter;

    /// Get a mut reference to the protocol adapter.
    fn protocol_adapter_mut(&mut self) -> &mut Self::ProtocolAdapter;

    /// Get a reference to the tx prover.
    fn prover(&self) -> &Self::Prover;

    /// Get a reference to a local commitment tree root.
    fn commitment_tree(&self) -> &MerkleTree;

    /// Get a mut reference to a local commitment tree root.
    fn commitment_tree_mut(&mut self) -> &mut MerkleTree;
}

// NOTE: `async_trait` trait is required to define a trait object
// compatible ERC-20 interface

/// ERC-20 interface.
#[async_trait]
pub trait Erc20 {
    /// Returns the value of tokens in existence.
    async fn total_supply(&self) -> anyhow::Result<U256>;

    /// Returns the value of tokens owned by `account`.
    async fn balance_of(&self, account: Address) -> anyhow::Result<U256>;

    /// Moves a `value` amount of tokens from the caller's account to `to`.
    ///
    /// Returns a boolean value indicating whether the operation succeeded.
    async fn transfer(&mut self, to: Address, value: U256) -> anyhow::Result<bool>;

    /// Returns the remaining number of tokens that `spender` will be
    /// allowed to spend on behalf of `owner` through {transferFrom}. This is
    /// zero by default.
    ///
    /// This value changes when `approve` or `transfer_from` are called.
    async fn allowance(&self, owner: Address, spender: Address) -> anyhow::Result<U256>;

    /// Sets a `value` amount of tokens as the allowance of `spender` over the
    /// caller's tokens.
    ///
    /// Returns a boolean value indicating whether the operation succeeded.
    async fn approve(&mut self, spender: Address, value: U256) -> anyhow::Result<bool>;

    /// Moves a `value` amount of tokens from `from` to `to` using the
    /// allowance mechanism. `value` is then deducted from the caller's
    /// allowance.
    ///
    /// Returns a boolean value indicating whether the operation succeeded.
    async fn transfer_from(
        &mut self,
        from: Address,
        to: Address,
        value: U256,
    ) -> anyhow::Result<bool>;

    /// Attempt to get the token's metadata.
    fn metadata(&self) -> Option<&dyn Erc20Metadata>;
}

/// Interface for the optional metadata functions from the ERC20 standard.
#[async_trait]
pub trait Erc20Metadata: Erc20 {
    /// Returns the name of the token.
    ///
    /// Example: "Wrapped Ether"
    async fn name(&self) -> anyhow::Result<Cow<'static, str>>;

    /// Returns the symbol of the token, usually a shorter version of the
    /// name (the moniker).
    ///
    /// Example: "WETH"
    async fn symbol(&self) -> anyhow::Result<Cow<'static, str>>;

    /// Returns the number of decimals used to get its user representation.
    ///
    /// For example, if `decimals` equals `2`, a balance of `505` tokens should
    /// be displayed to a user as `5.05` (`505 / 10 ** 2`).
    async fn decimals(&self) -> anyhow::Result<u8>;
}

/// ERC-20 token state.
pub trait Erc20Tokens {
    /// Look-up an ERC-20 token's address, by its symbol.
    fn address(&self, symbol: &str) -> anyhow::Result<Address>;

    /// Get a reference to an ERC-20 token.
    fn get(&self, erc20: Address) -> anyhow::Result<&dyn Erc20>;

    /// Get a mut reference to an ERC-20 token.
    fn get_mut(&mut self, erc20: Address) -> anyhow::Result<&mut dyn Erc20>;
}

/// Protocol adapter abstraction.
pub trait ProtocolAdapter {
    /// ARM transaction.
    type Transaction;

    /// Executes a transaction by adding the commitments and nullifiers
    /// to the commitment tree and nullifier set, respectively.
    async fn execute(&mut self, transaction: Self::Transaction) -> anyhow::Result<()>;

    /// Stops the protocol adapter permanently in case of an emergency.
    async fn emergency_stop(&mut self) -> anyhow::Result<()>;

    /// Returns whether the protocol adapter has been stopped or not.
    ///
    /// This can have two reasons:
    ///
    /// 1. The RISC Zero verifier associated with the protocol adapter has been stopped.
    /// 2. The protocol adapter itself was stopped by the owner.
    async fn is_emergency_stopped(&self) -> anyhow::Result<bool>;

    /// Check whether a nullifier is contained in the PA's state.
    async fn is_nullifier_contained(&self, nullifier: [u8; 32]) -> anyhow::Result<bool>;

    /// Returns the latest commitment tree root.
    async fn latest_commitment_tree_root(&self) -> anyhow::Result<Digest>;

    /// Return the number of nullifiers in the state.
    async fn nullifier_count(&self) -> anyhow::Result<usize>;

    /// Return the number of commitments in the state.
    async fn commitment_count(&self) -> anyhow::Result<usize>;
}

/// Transaction prover.
pub trait Prover {
    /// ARM transaction.
    type Transaction;

    /// Prove an ARM transaction.
    ///
    /// Invalid witnesses will result in an error.
    async fn prove(&self, actions: Vec<ActionWitnesses>) -> anyhow::Result<Self::Transaction>;
}
