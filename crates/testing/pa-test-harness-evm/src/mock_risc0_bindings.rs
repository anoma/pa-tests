use alloy::primitives::Address;
use alloy::providers::DynProvider;
use alloy::sol;

pub const MOCK_VERIFIER_SELECTOR: [u8; 4] = [0xFF, 0xFF, 0xFF, 0xFF];

sol!(
    #[allow(missing_docs)]
    #[derive(Debug)]
    #[sol(rpc)]
    MockRiscZeroVerifier,
    "artifacts/MockRiscZeroVerifier.json"
);

sol!(
    #[allow(missing_docs)]
    #[derive(Debug)]
    #[sol(rpc)]
    MockRiscZeroVerifierEmergencyStop,
    "artifacts/MockRiscZeroVerifierEmergencyStop.json"
);

sol!(
    #[allow(missing_docs)]
    #[derive(Debug)]
    #[sol(rpc)]
    MockRiscZeroVerifierRouter,
    "artifacts/MockRiscZeroVerifierRouter.json"
);

#[derive(Clone)]
pub struct MockRisc0Stack {
    pub router: MockRiscZeroVerifierRouter::MockRiscZeroVerifierRouterInstance<DynProvider>,
    pub emergency_stop:
        MockRiscZeroVerifierEmergencyStop::MockRiscZeroVerifierEmergencyStopInstance<DynProvider>,
    pub mock_verifier: MockRiscZeroVerifier::MockRiscZeroVerifierInstance<DynProvider>,
}

pub async fn deploy_mock_risc0_stack(
    provider: &DynProvider,
    guardian: Address,
) -> anyhow::Result<MockRisc0Stack> {
    let selector = alloy::primitives::FixedBytes::<4>::from(MOCK_VERIFIER_SELECTOR);

    let mock_verifier_deployed = MockRiscZeroVerifier::deploy(provider.clone(), selector).await?;
    let mock_verifier = MockRiscZeroVerifier::MockRiscZeroVerifierInstance::new(
        *mock_verifier_deployed.address(),
        provider.clone(),
    );

    let emergency_stop_deployed = MockRiscZeroVerifierEmergencyStop::deploy(
        provider.clone(),
        *mock_verifier.address(),
        guardian,
    )
    .await?;
    let emergency_stop =
        MockRiscZeroVerifierEmergencyStop::MockRiscZeroVerifierEmergencyStopInstance::new(
            *emergency_stop_deployed.address(),
            provider.clone(),
        );

    let router_deployed = MockRiscZeroVerifierRouter::deploy(provider.clone(), guardian).await?;
    let router = MockRiscZeroVerifierRouter::MockRiscZeroVerifierRouterInstance::new(
        *router_deployed.address(),
        provider.clone(),
    );

    router
        .addVerifier(selector, *emergency_stop.address())
        .send()
        .await?
        .get_receipt()
        .await?;

    Ok(MockRisc0Stack {
        router,
        emergency_stop,
        mock_verifier,
    })
}
