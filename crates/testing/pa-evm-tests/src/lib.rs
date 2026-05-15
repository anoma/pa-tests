pub type EvmIntegrationEnv = pa_test_harness_evm::envs::integration_tests::Environment;

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
