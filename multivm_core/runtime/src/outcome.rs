use multivm_primitives::Commitment;

pub struct ExecutionOutcome {
    pub session: risc0_zkvm::Session,
    pub commitment: Commitment,
    pub gas_used: u64,
    pub cross_calls_outcomes: Vec<ExecutionOutcome>,
}

impl ExecutionOutcome {
    pub fn new(
        session: risc0_zkvm::Session,
        gas_used: u64,
        cross_calls_outcomes: Vec<ExecutionOutcome>,
    ) -> Self {
        let commitment =
            Commitment::try_from_bytes(session.journal.clone()).expect("Corrupted journal");
        Self {
            session,
            commitment,
            gas_used,
            cross_calls_outcomes,
        }
    }

    pub fn prove_all(&self) -> ProvedExecutionOutcome {
        let cross_calls_outcomes = self
            .cross_calls_outcomes
            .iter()
            .map(|outcome| outcome.prove_all())
            .collect();

        let receipt = self.session.prove().expect("Failed to prove");
        ProvedExecutionOutcome::new(
            receipt,
            self.commitment.clone(),
            self.gas_used,
            cross_calls_outcomes,
        )
    }
}

pub struct ProvedExecutionOutcome {
    pub receipt: risc0_zkvm::Receipt,
    pub commitment: Commitment,
    pub gas_used: u64,
    pub cross_calls_outcomes: Vec<ProvedExecutionOutcome>,
}

impl ProvedExecutionOutcome {
    pub fn new(
        receipt: risc0_zkvm::Receipt,
        commitment: Commitment,
        gas_used: u64,
        cross_calls_outcomes: Vec<ProvedExecutionOutcome>,
    ) -> Self {
        Self {
            receipt,
            commitment,
            gas_used,
            cross_calls_outcomes,
        }
    }
}
