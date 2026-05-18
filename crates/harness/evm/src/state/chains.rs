use alloy_chains::NamedChain;
use pa_test_harness_core::environment::Environment;
use pa_test_harness_core::environment::State;
use pa_test_harness_core::environment::StateBuilder;

use crate::state::keys::chain_id_key;

#[inline]
pub fn insert_chain_id(builder: &mut StateBuilder, chain: NamedChain) {
    builder.insert(chain_id_key(chain), chain as u64);
}

#[inline]
pub fn chain_id<E>(env: &E, chain: NamedChain) -> anyhow::Result<u64>
where
    E: Environment,
{
    chain_id_in_state(env.state(), chain)
}

#[inline]
pub fn chain_id_in_state(state: &State, chain: NamedChain) -> anyhow::Result<u64> {
    state.get::<u64>(&chain_id_key(chain)).copied()
}

#[cfg(test)]
mod tests {
    use pa_test_harness_core::environment::StateBuilder;
    use pa_test_harness_core::mocks::MockEnvironment;

    use super::*;

    #[test]
    fn insert_and_resolve_chain_id_for_named_chain() {
        let mut builder = StateBuilder::new();
        insert_chain_id(&mut builder, NamedChain::Sepolia);

        let state = builder.finalize();

        let mut env = MockEnvironment::new();
        env.expect_state().return_const(state);

        let resolved = chain_id(&env, NamedChain::Sepolia).expect("must resolve chain id");
        assert_eq!(resolved, 11155111);
    }

    #[test]
    fn insert_chain_id_uses_named_chain_identifier() {
        let mut builder = StateBuilder::new();
        insert_chain_id(&mut builder, NamedChain::Mainnet);

        let state = builder.finalize();

        let mut env = MockEnvironment::new();
        env.expect_state().return_const(state);

        let resolved = chain_id(&env, NamedChain::Mainnet).expect("must resolve chain id");
        assert_eq!(resolved, NamedChain::Mainnet as u64);
    }

    #[test]
    fn missing_chain_id_key_errors() {
        let mut builder = StateBuilder::new();
        insert_chain_id(&mut builder, NamedChain::Mainnet);

        let state = builder.finalize();

        let mut env = MockEnvironment::new();
        env.expect_state().return_const(state);

        let err = chain_id(&env, NamedChain::Sepolia).expect_err("must fail on missing key");
        assert!(
            err.to_string().contains("evm.chain.id.sepolia"),
            "error should mention missing key, got: {err}"
        );
    }
}
