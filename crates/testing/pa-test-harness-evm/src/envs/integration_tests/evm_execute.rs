use alloy::providers::DynProvider;
use anoma_pa_evm_bindings::generated::protocol_adapter::ProtocolAdapter as PaContract;

pub(super) async fn execute_on_pa(
    pa: &PaContract::ProtocolAdapterInstance<DynProvider>,
    tx: PaContract::Transaction,
) -> anyhow::Result<alloy::rpc::types::TransactionReceipt> {
    let receipt = pa.execute(tx).send().await?.get_receipt().await?;

    if !receipt.status() {
        return Err(anyhow::anyhow!("protocol adapter execution failed"));
    }

    Ok(receipt)
}
