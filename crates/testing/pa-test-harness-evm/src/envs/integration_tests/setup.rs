use alloy::node_bindings::Anvil;
use alloy::primitives::b256;
use alloy::primitives::utils::parse_ether;
use alloy::providers::Provider;
use alloy::providers::ProviderBuilder;
use alloy::providers::ext::AnvilApi;
use alloy_chains::NamedChain;
use anyhow::Context;
use pa_test_harness_core::environment::StateBuilder;

use crate::pa::{deploy_protocol_adapter, protocol_adapter};
use crate::state::actors::insert_default_signer;
use crate::state::chains::insert_chain_id;
use crate::state::pa::insert_pa_address;

use super::{CommitmentTree, Environment, ProtocolAdapter, Prover};

impl Environment {
    pub async fn setup() -> anyhow::Result<Self> {
        let anvil = Anvil::new().spawn();

        let signer = alloy::signers::local::PrivateKeySigner::from_bytes(&b256!(
            "7ad4b84636a3fa408827e7202f6da39287bbf099d1fab6250d3b56e03e77586b"
        ))?;
        let deployer = signer.address();

        let provider = ProviderBuilder::new()
            .wallet(signer)
            .connect_http(anvil.endpoint_url())
            .erased();

        provider
            .anvil_set_balance(
                deployer,
                parse_ether("100").context("failed to parse deployer balance amount")?,
            )
            .await?;

        let pa_address = deploy_protocol_adapter(&provider, deployer).await?;
        let pa = protocol_adapter(pa_address, provider.clone());

        let chain_id = provider.get_chain_id().await?;
        let named_chain = NamedChain::try_from(chain_id)
            .with_context(|| format!("unsupported chain id {chain_id}"))?;

        let state = {
            let mut builder = StateBuilder::new();

            insert_default_signer(&mut builder, provider.clone());
            insert_chain_id(&mut builder, named_chain);
            insert_pa_address(&mut builder, pa_address);

            builder.finalize()
        };

        Ok(Self {
            anvil,
            state,
            prover: Prover,
            protocol_adapter: ProtocolAdapter {
                pa,
                commitment_tree: CommitmentTree::default(),
            },
        })
    }
}
