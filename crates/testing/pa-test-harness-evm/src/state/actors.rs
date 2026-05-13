use alloy::providers::DynProvider;
use anyhow::Context;
use pa_test_harness_core::environment::Environment;
use pa_test_harness_core::environment::State;
use pa_test_harness_core::environment::StateBuilder;

pub const KEY_DEFAULT_SIGNER: &str = "evm.actor.default_signer";

#[inline]
pub fn insert_default_signer(builder: &mut StateBuilder, provider: DynProvider) {
    builder.insert(KEY_DEFAULT_SIGNER, provider);
}

#[inline]
pub fn default_signer<E>(env: &E) -> anyhow::Result<DynProvider>
where
    E: Environment,
{
    default_signer_in_state(env.state())
}

#[inline]
pub fn default_signer_in_state(state: &State) -> anyhow::Result<DynProvider> {
    state
        .get::<DynProvider>(KEY_DEFAULT_SIGNER)
        .cloned()
        .context("failed to retrieve default signer from env")
}

#[cfg(test)]
mod tests {
    use alloy::providers::Provider;
    use alloy::providers::ProviderBuilder;
    use pa_test_harness_core::mocks::MockEnvironment;

    use super::*;

    #[test]
    fn default_signer_key_format() {
        assert_eq!(KEY_DEFAULT_SIGNER, "evm.actor.default_signer");
    }

    #[test]
    fn insert_and_resolve_default_signer() {
        let provider = ProviderBuilder::new()
            .connect_http(
                "http://127.0.0.1:8545"
                    .parse()
                    .expect("valid local rpc url"),
            )
            .erased();

        let state = {
            let mut builder = StateBuilder::new();
            insert_default_signer(&mut builder, provider.clone());
            builder.finalize()
        };

        let mut env = MockEnvironment::new();
        env.expect_state().return_const(state);

        let _resolved = default_signer(&env).expect("must resolve stored default signer");
    }

    #[test]
    fn missing_default_signer_errors() {
        let state = StateBuilder::new().finalize();

        let mut env = MockEnvironment::new();
        env.expect_state().return_const(state);

        let err = default_signer(&env).expect_err("must fail on missing default signer key");
        assert!(
            err.to_string()
                .contains("failed to retrieve default signer from env"),
            "error should mention missing default signer, got: {err}"
        );
    }
}
