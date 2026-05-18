pub type EvmIntegrationEnv = pa_test_harness_evm::envs::integration_test::Environment;
pub type EvmIntegrationTx = pa_test_harness_evm::envs::integration_test::Transaction;

use anyhow::Context;

pub use pa_test_harness_core::{commitment_root, execute_tx, prove_actions};

#[derive(Debug)]
pub enum Needle {
    Static(&'static str),
    Regexp(regex::Regex),
}

pub fn expect_integration_panic<T>(
    needle: Needle,
) -> impl FnOnce(anyhow::Result<T>) -> anyhow::Result<()> {
    move |result| {
        let Err(error) = result else {
            anyhow::bail!("expected to find error {needle:?}, but got anyhow::Ok");
        };

        let dbg_error = format!("{error:?}");
        let found_needle = match &needle {
            Needle::Static(s) => dbg_error.contains(s),
            Needle::Regexp(re) => re.is_match(&dbg_error),
        };

        if !found_needle {
            return Err(error.context(format!("couldn't find needle {needle:?} in error")));
        }

        Ok(())
    }
}

pub fn tamper_integration_first_logic_seal(tx: &mut EvmIntegrationTx) -> anyhow::Result<()> {
    let logic_input = tx
        .as_arm_mut()
        .actions
        .first_mut()
        .context("tamper requires at least one action")?
        .logic_verifier_inputs
        .first_mut()
        .context("tamper requires at least one logic verifier input")?;

    let proof = logic_input
        .proof
        .as_mut()
        .context("tamper requires first logic proof")?;

    let mut inner: risc0_zkvm::InnerReceipt = bincode::deserialize(proof)
        .context("tamper requires bincode-encoded inner receipt proof")?;

    let receipt = match &mut inner {
        risc0_zkvm::InnerReceipt::Groth16(receipt) => receipt,
        _ => anyhow::bail!("tamper requires Groth16 inner receipt proof"),
    };

    let byte = receipt
        .seal
        .first_mut()
        .context("tamper requires non-empty inner seal")?;
    *byte ^= 0x01;

    *proof =
        bincode::serialize(&inner).context("tamper must re-serialize modified inner receipt")?;

    Ok(())
}
