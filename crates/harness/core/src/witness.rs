use anoma_rm_risc0::Digest;
use anoma_rm_risc0::compliance::ComplianceWitness;
use anoma_rm_risc0::logic_instance::LogicInstance;
use anoma_rm_risc0::logic_proof::LogicProver;
use anoma_rm_risc0::resource_logic::LogicCircuit;
use anyhow::Context;

/// Witness data of an individual action.
pub struct ActionWitnesses {
    /// Steps of the action.
    pub compliance_units: Vec<ComplianceUnitWitnesses>,
}

/// Individual step of an action.
pub struct ComplianceUnitWitnesses {
    /// Witness of the compliance unit.
    pub compliance_witness: Box<ComplianceWitness>,
    /// Consumed logic instance witness.
    pub consumed_logic_witness: Box<dyn LogicWitness>,
    /// Created logic instance witness.
    pub created_logic_witness: Box<dyn LogicWitness>,
}

impl ComplianceUnitWitnesses {
    pub fn new<Consumed, Created>(
        compliance: ComplianceWitness,
        consumed: Consumed,
        created: Created,
    ) -> Self
    where
        Consumed: LogicWitness + 'static,
        Created: LogicWitness + 'static,
    {
        Self {
            compliance_witness: Box::new(compliance),
            consumed_logic_witness: Box::new(consumed),
            created_logic_witness: Box::new(created),
        }
    }
}

/// Witness of a logic proof.
pub trait LogicWitness {
    /// Verifying key of the circuit.
    fn verifying_key(&self) -> Digest;

    /// Constrain the circuit, yielding a logic instance.
    fn constrain(&self) -> anyhow::Result<LogicInstance>;

    /// Serialize the witness to RISC-V words for remote proving.
    fn witness_to_vec(&self) -> anyhow::Result<Vec<u32>>;

    /// Proving key of the circuit.
    fn proving_key(&self) -> Vec<u8>;
}

impl LogicWitness for Box<dyn LogicWitness> {
    #[inline]
    fn verifying_key(&self) -> Digest {
        (**self).verifying_key()
    }

    #[inline]
    fn constrain(&self) -> anyhow::Result<LogicInstance> {
        (**self).constrain()
    }

    #[inline]
    fn witness_to_vec(&self) -> anyhow::Result<Vec<u32>> {
        (**self).witness_to_vec()
    }

    #[inline]
    fn proving_key(&self) -> Vec<u8> {
        (**self).proving_key()
    }
}

impl<W> LogicWitness for W
where
    W: LogicProver + LogicCircuit,
{
    #[inline]
    fn verifying_key(&self) -> Digest {
        <W as LogicProver>::verifying_key()
    }

    #[inline]
    fn constrain(&self) -> anyhow::Result<LogicInstance> {
        <W as LogicCircuit>::constrain(self)
            .with_context(|| format!("invalid proof of {} witness", std::any::type_name::<W>()))
    }

    #[inline]
    fn witness_to_vec(&self) -> anyhow::Result<Vec<u32>> {
        risc0_zkvm::serde::to_vec(<W as LogicProver>::witness(self)).with_context(|| {
            format!(
                "failed to serialize {} witness to risc0 words",
                std::any::type_name::<W>()
            )
        })
    }

    #[inline]
    fn proving_key(&self) -> Vec<u8> {
        <W as LogicProver>::proving_key().to_vec()
    }
}
