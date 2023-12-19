use multivm_primitives::Commitment;

pub struct ExecutionOutcome {
    pub session_info: risc0_zkvm::SessionInfo,
    pub commitment: Commitment,
    pub gas_used: u64,
    pub cross_calls_outcomes: Vec<ExecutionOutcome>,
}

impl ExecutionOutcome {
    pub fn new(
        session_info: risc0_zkvm::SessionInfo,
        gas_used: u64,
        cross_calls_outcomes: Vec<ExecutionOutcome>,
    ) -> Self {
        let commitment = Commitment::try_from_bytes(session_info.journal.bytes.clone())
            .expect("Corrupted journal");
        Self {
            session_info,
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

        ProvedExecutionOutcome::new(
            (),
            self.commitment.clone(),
            self.gas_used,
            cross_calls_outcomes,
        )
    }
}

pub struct ProvedExecutionOutcome {
    pub receipt: (),
    pub commitment: Commitment,
    pub gas_used: u64,
    pub cross_calls_outcomes: Vec<ProvedExecutionOutcome>,
}

impl ProvedExecutionOutcome {
    pub fn new(
        receipt: (), // TODO: prove session, create receipt
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
