use std::str::FromStr;

use borsh::{BorshDeserialize, BorshSerialize};
use eth_primitive_types::{H160, U256};
use ethabi::ethereum_types::U64;
use ethers_core::{
    types::{Signature, TransactionRequest},
    utils::rlp::Rlp,
};
use k256::ecdsa::{SigningKey, VerifyingKey};
use multivm_primitives::{
    AccountId, Commitment, ContractCall, ContractCallContext, EnvironmentContext, EvmAddress,
    MultiVmAccountId, SignedTransaction, SupportedTransaction,
};
use multivm_runtime::viewer::SupportedView;
use playgrounds::NodeHelper;
use tracing::info;

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Pool {
    pub token0: Token,
    pub token1: Token,
    pub reserve0: u128,
    pub reserve1: u128,
    pub k_last: u128, // reserve0 * reserve1
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Token {
    pub symbol: String,
    pub address: String,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct AddLiquidity {
    pub pool_id: u128,
    pub amount0: u128,
    pub amount1: u128,
}

#[derive(BorshSerialize, BorshDeserialize)]
pub struct Swap {
    pub pool_id: u128,
    pub amount0_in: u128,
    pub amount1_in: u128,
}

fn main() {
    playgrounds::install_tracing();

    let mut helper = NodeHelper::new_temp();

    let token0_address = "usdt.multivm";
    let token1_address = "ethereum.multivm";

    let alice_id = MultiVmAccountId::try_from("alice.multivm").unwrap();
    let nikita_id = MultiVmAccountId::try_from("nikita.multivm").unwrap();
    let token0_id = MultiVmAccountId::try_from(token0_address.clone()).unwrap();
    let token1_id = MultiVmAccountId::try_from(token1_address.clone()).unwrap();
    let amm_id = MultiVmAccountId::try_from("amm.multivm").unwrap();

    let alice: AccountId = alice_id.clone().into();
    let nikita: AccountId = nikita_id.clone().into();
    let token0: AccountId = token0_id.clone().into();
    let token1: AccountId = token1_id.clone().into();
    let amm: AccountId = amm_id.clone().into();

    // for account_id in vec![alice_id.clone(), nikita_id.clone()] {
    //     helper.create_account(&account_id);

    //     helper.produce_block(true);
    //     let _account = helper.account(&account_id.clone().into());
    //     info!("===== Account created:: {}", account_id);
    // }

    // let token_code = include_bytes!(
    //     "../../../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/token_contract"
    // )
    // .to_vec();

    // for (token, ticker) in vec![(token0.clone(), "USDT"), (token1.clone(), "ETH")] {
    //     helper.create_contract(&token.multivm(), token_code.clone());
    //     helper.produce_block(true);

    //     helper.call_contract(
    //         &token,
    //         &token,
    //         ContractCall::new_call("init", &(ticker.to_string(), 100000000u128)),
    //     );
    //     helper.produce_block(true);

    //     helper.call_contract(
    //         &token,
    //         &token,
    //         ContractCall::new_call("transfer", &(nikita.clone(), 10000000u128)),
    //     );
    //     helper.produce_block(true);
    // }

    // info!("===== Tokens and transfers completed");

    // let amm_code = include_bytes!(
    //     "../../../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/amm"
    // )
    // .to_vec();
    // helper.create_contract(&amm_id, amm_code);
    // helper.produce_block(true);

    let user1_id = MultiVmAccountId::try_from("user1.multivm").unwrap();
    let private_key = "af1a53abf88f4821840a2934f3facfc8b1827cccd7f2e331375d2faf8c1032d2";
    let pk_bytes = hex::decode(private_key).unwrap();
    let pk = SigningKey::from_slice(&pk_bytes).unwrap();
    let vk = pk.verifying_key().clone();

    helper.create_evm_account(&user1_id, vk);

    info!("=== EVM account created");

    let amm_code = include_bytes!(
        "../../../../example_contracts/target/riscv-guest/riscv32im-risc0-zkvm-elf/release/amm"
    )
    .to_vec();
    helper.deploy_contract(&user1_id, amm_code);
    helper.produce_block(true);

    info!("=== AMM code deployed");

    let call = ContractCall::new_call("init", &());
    let mut call_bytes: Vec<u8> = Vec::new();
    call.serialize(&mut call_bytes).unwrap();

    let from = H160::from_str("0x121348F398681B4d021826EB6423805df7CD25D9").unwrap();
    let tx_request = TransactionRequest::new()
        .from(from)
        .to(from)
        .data(call_bytes);

    let (ecdsa_sig, rec_id) = pk
        .sign_prehash_recoverable(&tx_request.sighash().as_bytes())
        .unwrap();
    let sig = Signature {
        r: U256::from_str_radix(format!("{}", ecdsa_sig.r()).as_str(), 16).unwrap(),
        s: U256::from_str_radix(format!("{}", ecdsa_sig.s()).as_str(), 16).unwrap(),
        v: rec_id.to_byte() as u64,
    };

    // sending EVM tx with MultiVM call to MultiVM account
    let tx = SupportedTransaction::Evm(tx_request.rlp_signed(&sig).to_vec());
    let hash = tx.hash();
    helper.node.add_tx(tx);
    let block = helper.produce_block(true);
    let output = block.call_outputs.get(&hash).unwrap();

    let commitment: Commitment = BorshDeserialize::deserialize(&mut output.as_slice()).unwrap();
    let result: String = String::from_utf8(commitment.response).unwrap();
    info!("{}", result);

    return;

    helper.call_contract(&nikita, &amm, ContractCall::new_call("init", &()));
    helper.produce_block(true);

    info!("======= Initialized");

    let hash = helper.call_contract(
        &nikita,
        &amm,
        ContractCall::new_call(
            "add_pool",
            &(token0_address.to_string(), token1_address.to_string()),
        ),
    );
    let block = helper.produce_block(true);
    let output = block.call_outputs.get(&hash).unwrap();
    let pool_id: u128 = BorshDeserialize::deserialize(&mut output.as_slice()).unwrap();

    info!("======= created pool: {}", pool_id);

    helper.call_contract(
        &nikita,
        &amm,
        ContractCall::new_call(
            "add_liquidity",
            &AddLiquidity {
                pool_id,
                amount0: 3 * 2085 * 1000,
                amount1: 3 * 1 * 1000,
            },
        ),
    );
    helper.produce_block(true);

    info!("======= added liquidity");

    for token in vec![token0.clone(), token1.clone()] {
        let view = SupportedView::MultiVm(ContractCallContext {
            contract_id: token.clone(),
            contract_call: ContractCall::new_call("balance_of", &nikita),
            sender_id: nikita.clone(),
            signer_id: nikita.clone(),
            environment: EnvironmentContext {
                block_height: helper.node.latest_block().height,
            },
        });

        let balance_bytes = helper.node.view(view);
        let balance: u128 = BorshDeserialize::deserialize(&mut balance_bytes.as_slice()).unwrap();
        info!(
            "======== Balance of {:#?} is now {}",
            token.multivm(),
            balance
        );
    }

    helper.call_contract(
        &nikita,
        &amm,
        ContractCall::new_call(
            "swap",
            &Swap {
                pool_id,
                amount0_in: 100 * 1000,
                amount1_in: 0 * 1000,
            },
        ),
    );
    helper.produce_block(true);

    info!("======= swapped");

    for token in vec![token0.clone(), token1.clone()] {
        let view = SupportedView::MultiVm(ContractCallContext {
            contract_id: token.clone(),
            contract_call: ContractCall::new_call("balance_of", &nikita),
            sender_id: nikita.clone(),
            signer_id: nikita.clone(),
            environment: EnvironmentContext {
                block_height: helper.node.latest_block().height,
            },
        });

        let balance_bytes = helper.node.view(view);
        let balance: u128 = BorshDeserialize::deserialize(&mut balance_bytes.as_slice()).unwrap();
        info!(
            "======== Balance of {:#?} after swap {}",
            token.multivm(),
            balance
        );
    }
}
