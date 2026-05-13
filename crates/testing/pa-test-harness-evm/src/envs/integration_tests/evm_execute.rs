use alloy::providers::DynProvider;
use anoma_pa_evm_bindings::generated::protocol_adapter::ProtocolAdapter as PaContract;
use anyhow::Context;

pub(super) async fn execute_on_pa(
    pa: &PaContract::ProtocolAdapterInstance<DynProvider>,
    tx: PaContract::Transaction,
) -> anyhow::Result<alloy::rpc::types::TransactionReceipt> {
    let receipt = pa
        .execute(tx)
        .send()
        .await
        .context("failed to submit protocol adapter execution transaction")?
        .get_receipt()
        .await
        .context("failed to fetch protocol adapter execution receipt")?;

    if !receipt.status() {
        return Err(anyhow::anyhow!("protocol adapter execution failed"));
    }

    Ok(receipt)
}
