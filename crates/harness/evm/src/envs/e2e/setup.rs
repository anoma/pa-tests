use alloy::node_bindings::Anvil;
use alloy::primitives::b256;
use alloy::primitives::utils::parse_ether;
use alloy::providers::Provider;
use alloy::providers::ProviderBuilder;
use alloy::providers::ext::AnvilApi;
use alloy_chains::NamedChain;
use anyhow::Context;
use heliax_ap_orchestrator_sdk::QueueClient;
use pa_test_harness_core::environment::StateBuilder;

use crate::state::actors::insert_default_signer;
use crate::state::chains::insert_chain;
use crate::state::pa::insert_pa_address;

use super::CommitmentTree;
use super::Environment;
use super::ProtocolAdapter;
use super::Prover;
use super::config::E2eConfig;

impl Environment {
    pub async fn setup(config: E2eConfig) -> anyhow::Result<Self> {
        Self::setup_with_additional(config, async |_| anyhow::Ok(())).await
    }

    pub async fn setup_with_additional<F>(
        config: E2eConfig,
        insert_additional: F,
    ) -> anyhow::Result<Self>
    where
        F: AsyncFnOnce(&mut StateBuilder) -> anyhow::Result<()>,
    {
        let fork_url = format!(
            "https://eth-sepolia.g.alchemy.com/v2/{}",
            config.alchemy_api_key
        );
        let anvil = Anvil::new().fork(fork_url).spawn();

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

        let pa_address = deploy_fresh_pa(&provider, deployer).await?;

        let pa = anoma_pa_evm_bindings::generated::protocol_adapter::ProtocolAdapter::ProtocolAdapterInstance::new(pa_address, provider.clone());

        let chain_id = provider.get_chain_id().await?;
        let named_chain = NamedChain::try_from(chain_id)
            .with_context(|| format!("unsupported chain id {chain_id}"))?;

        let mut builder = QueueClient::builder(&config.queue_base_url);
        if let Some(token) = &config.queue_auth_token {
            builder = builder.auth_token(token);
        }
        let queue_client = builder.build().context("failed to build queue client")?;

        let state = {
            let mut builder = StateBuilder::new();

            insert_default_signer(&mut builder, provider.clone());
            insert_chain(&mut builder, named_chain);
            insert_pa_address(&mut builder, pa_address);
            insert_additional(&mut builder)
                .await
                .context("failed to insert additional data into state")?;

            builder.finalize()
        };

        Ok(Self {
            anvil,
            state,
            prover: Prover {
                queue: queue_client,
            },
            protocol_adapter: ProtocolAdapter {
                pa,
                commitment_tree: CommitmentTree::default(),
            },
        })
    }
}

async fn deploy_fresh_pa(
    provider: &alloy::providers::DynProvider,
    fee_recipient: alloy::primitives::Address,
) -> anyhow::Result<alloy::primitives::Address> {
    use anoma_pa_evm_bindings::contract::protocol_adapter as get_reference_pa;
    use anoma_pa_evm_bindings::generated::protocol_adapter::ProtocolAdapter as PaContract;

    let reference_pa = get_reference_pa(provider)
        .await
        .context("failed to get reference protocol adapter on forked chain")?;

    let verifier_router = reference_pa
        .getRiscZeroVerifierRouter()
        .call()
        .await
        .context("failed to get verifier router from reference PA")?;

    let verifier_selector = reference_pa
        .getRiscZeroVerifierSelector()
        .call()
        .await
        .context("failed to get verifier selector from reference PA")?;

    let deployed = PaContract::deploy(
        provider.clone(),
        verifier_router,
        verifier_selector,
        fee_recipient,
    )
    .await
    .context("failed to deploy protocol adapter")?;

    Ok(*deployed.address())
}
