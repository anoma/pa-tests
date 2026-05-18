use alloy::node_bindings::Anvil;
use alloy::providers::Provider;
use alloy::providers::ProviderBuilder;

use pa_test_harness_evm_mock_permit2::deploy_permit2_canonical;

#[tokio::test]
async fn permit2_creation_bytecode_deploys_expected_runtime_code() {
    let anvil = Anvil::new().spawn();
    let provider = ProviderBuilder::new()
        .wallet(
            anvil
                .wallet()
                .expect("failed to get eth wallet from anvil instance"),
        )
        .connect_http(anvil.endpoint_url())
        .erased();

    deploy_permit2_canonical(&provider)
        .await
        .expect("failed to deploy permit2");
}
