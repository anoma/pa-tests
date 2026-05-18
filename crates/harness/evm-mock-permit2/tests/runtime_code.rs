use alloy::node_bindings::Anvil;
use alloy::providers::Provider;
use alloy::providers::ProviderBuilder;
use alloy::rpc::types::TransactionInput;
use alloy::rpc::types::TransactionRequest;

use pa_test_harness_evm_mock_permit2::PERMIT2_RUNTIME_CODE_HASH;
use pa_test_harness_evm_mock_permit2::permit2_creation_bytecode;

#[tokio::test]
async fn permit2_creation_bytecode_deploys_expected_runtime_code() {
    let anvil = Anvil::new().spawn();
    let provider = ProviderBuilder::new()
        .connect_http(anvil.endpoint_url())
        .erased();

    let creation = permit2_creation_bytecode().expect("creation bytecode must decode");

    let deploy_tx = TransactionRequest::default()
        .input(TransactionInput::both(creation))
        .from(anvil.addresses()[0]);

    let receipt = provider
        .send_transaction(deploy_tx)
        .await
        .expect("deployment tx must submit")
        .get_receipt()
        .await
        .expect("deployment receipt must be available");

    let contract = receipt
        .contract_address
        .expect("create tx receipt must include contract address");

    let runtime = provider
        .get_code_at(contract)
        .await
        .expect("must fetch deployed runtime code");

    assert_eq!(
        alloy::primitives::keccak256(runtime),
        PERMIT2_RUNTIME_CODE_HASH
    );
}
