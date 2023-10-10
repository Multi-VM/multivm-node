use std::time::Duration;

use multivm_primitives::{AccountId, ContractCall, MultiVmAccountId};
use playgrounds::NodeHelper;

fn main() {
    // playgrounds::install_tracing();

    let benches: Vec<(&str, Box<dyn Fn(bool) -> Duration>)> = vec![
        ("account_creation_1", Box::new(account_creation_1)),
        ("account_creation_5", Box::new(account_creation_5)),
        ("account_creation_20", Box::new(account_creation_20)),
        ("fibonacci_creation", Box::new(fibonacci_creation)),
        ("fibonacci_full_flow", Box::new(fibonacci_full_flow)),
        (
            "token_contract_transfer_1",
            Box::new(token_contract_transfer_1),
        ),
        (
            "token_contract_transfer_10",
            Box::new(token_contract_transfer_10),
        ),
    ];

    for bench in benches {
        for skip_proof in [true, false] {
            let t = bench.1(skip_proof);
            println!(
                "{} ({}): {:.2}s",
                bench.0,
                if skip_proof {
                    "without proof"
                } else {
                    "with proof"
                },
                t.as_secs_f64()
            )
        }
        println!("")
    }
}

fn account_creation_1(skip_proof: bool) -> Duration {
    let mut helper = NodeHelper::new_temp();
    let alice_id = MultiVmAccountId::try_from("alice.multivm").unwrap();
    helper.create_account(&alice_id);

    let start = std::time::Instant::now();
    helper.produce_block(skip_proof);
    start.elapsed()
}

fn account_creation_5(skip_proof: bool) -> Duration {
    let mut helper = NodeHelper::new_temp();
    for i in 0..5 {
        helper.create_account(
            &MultiVmAccountId::try_from(format!("{}.multivm", i))
                .unwrap()
                .into(),
        );
    }

    let start = std::time::Instant::now();
    helper.produce_block(skip_proof);
    start.elapsed()
}

fn account_creation_20(skip_proof: bool) -> Duration {
    let mut helper = NodeHelper::new_temp();
    for i in 0..20 {
        helper.create_account(
            &MultiVmAccountId::try_from(format!("{}.multivm", i))
                .unwrap()
                .into(),
        );
    }

    let start = std::time::Instant::now();
    helper.produce_block(skip_proof);
    start.elapsed()
}

fn fibonacci_creation(skip_proof: bool) -> Duration {
    let mut helper = NodeHelper::new_temp();
    let code =  include_bytes!("../../../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/fibonacci_contract").to_vec();
    helper.create_contract(
        &MultiVmAccountId::try_from("fibonacci.multivm")
            .unwrap()
            .into(),
        code,
    );

    let start = std::time::Instant::now();
    helper.produce_block(skip_proof);
    start.elapsed()
}

fn fibonacci_full_flow(skip_proof: bool) -> Duration {
    let mut helper = NodeHelper::new_temp();
    let fibonacci_id = MultiVmAccountId::try_from("fibonacci.multivm")
        .unwrap()
        .into();
    let alice_id = MultiVmAccountId::try_from("alice.multivm").unwrap().into();

    let code =  include_bytes!("../../../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/fibonacci_contract").to_vec();
    helper.create_contract(&fibonacci_id, code);
    helper.create_account(&alice_id);

    helper.call_contract(
        &alice_id.into(),
        &fibonacci_id.into(),
        ContractCall::new("fibonacci".into(), &10u32, 300_000, 0),
    );

    let start = std::time::Instant::now();
    helper.produce_block(skip_proof);
    let elapsed = start.elapsed();

    elapsed
}

fn token_contract_transfer_1(skip_proof: bool) -> Duration {
    let mut helper = NodeHelper::new_temp();
    let token_id = MultiVmAccountId::try_from("token.multivm").unwrap();
    let alice_id = MultiVmAccountId::try_from("alice.multivm").unwrap();
    let bob_id = MultiVmAccountId::try_from("bob.multivm").unwrap();

    let code =  include_bytes!("../../../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/token_contract").to_vec();
    helper.create_contract(&token_id, code);
    helper.create_account(&alice_id);
    helper.create_account(&bob_id);
    helper.call_contract(
        &alice_id.clone().into(),
        &token_id.clone().into(),
        ContractCall::new("init".into(), &(String::from("TOKEN"), 100u128), 300_000, 0),
    );
    helper.produce_block(true);

    helper.call_contract(
        &alice_id.into(),
        &token_id.into(),
        ContractCall::new(
            "transfer".into(),
            &(AccountId::from(bob_id.clone()), 50u128),
            300_000,
            0,
        ),
    );

    let start = std::time::Instant::now();
    helper.produce_block(skip_proof);
    let elapsed = start.elapsed();

    elapsed
}

fn token_contract_transfer_10(skip_proof: bool) -> Duration {
    let mut helper = NodeHelper::new_temp();
    let token_id = MultiVmAccountId::try_from("token.multivm").unwrap();
    let alice_id = MultiVmAccountId::try_from("alice.multivm").unwrap();
    let bob_id = MultiVmAccountId::try_from("bob.multivm").unwrap();

    let code =  include_bytes!("../../../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/token_contract").to_vec();
    helper.create_contract(&token_id, code);
    helper.create_account(&alice_id);
    helper.create_account(&bob_id);
    helper.call_contract(
        &alice_id.clone().into(),
        &token_id.clone().into(),
        ContractCall::new(
            "init".into(),
            &(String::from("TOKEN"), 100_000u128),
            300_000,
            0,
        ),
    );
    helper.produce_block(true);

    for i in 0..10 {
        helper.call_contract(
            &alice_id.clone().into(),
            &token_id.clone().into(),
            ContractCall::new(
                "transfer".into(),
                &(AccountId::from(bob_id.clone()), 50u128 + i),
                300_000,
                0,
            ),
        );
    }

    let start = std::time::Instant::now();
    helper.produce_block(skip_proof);
    let elapsed = start.elapsed();

    elapsed
}
