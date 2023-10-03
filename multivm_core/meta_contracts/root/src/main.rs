#![no_main]

use core::panic;

use borsh::{BorshDeserialize, BorshSerialize};
use multivm_primitives::{
    AccountId, ContractCall, ContractCallContext, SignedTransaction,
    SYSTEM_META_CONTRACT_ACCOUNT_ID,
};

mod system_env;

const TOKEN_DECIMALS: u32 = 8;
const ONE_TOKEN: u128 = 10u128.pow(TOKEN_DECIMALS);

// TODO: move to multivm_primitives
#[derive(BorshDeserialize, BorshSerialize)]
struct Account {
    pub account_id: AccountId,
    pub public_key: Vec<u8>,
    pub image_id: Option<[u32; 8]>,
    pub balance: u128,
    pub nonce: u64,
}

#[derive(BorshDeserialize, BorshSerialize)]
struct AccountCreationRequest {
    pub account_id: AccountId,
    pub public_key: Vec<u8>,
}

#[derive(BorshDeserialize, BorshSerialize)]
struct ContractDeploymentArgs {
    pub image_id: [u32; 8],
}

#[derive(BorshDeserialize, BorshSerialize)]
enum Action {
    ExecuteTransaction(SignedTransaction),
    View(ContractCallContext),
}

risc0_zkvm::entry!(entrypoint);

fn entrypoint() {
    let mut bytes: Vec<u8> = risc0_zkvm::guest::env::read();

    let action: Action = BorshDeserialize::try_from_slice(&mut bytes).expect("Corrupted action");

    match action {
        Action::ExecuteTransaction(signed_tx) => process_transaction(signed_tx),
        Action::View(context) => view(context),
    };
}

fn view(context: ContractCallContext) {
    system_env::setup_env(&context);

    match context.contract_call.method.as_str() {
        "account_info" => account_info(context),
        _ => panic!("Method not found"),
    }
}

fn process_transaction(signed_tx: SignedTransaction) {
    let ctx = signed_tx.transaction.context(0);

    system_env::setup_env(&ctx);

    let signer_id = system_env::signer();
    if signer_id.to_string() == SYSTEM_META_CONTRACT_ACCOUNT_ID {
        if let None = get_account(signer_id) {
            let account = Account {
                account_id: system_env::signer(),
                public_key: vec![],
                image_id: None,
                balance: 1_000_000_000 * ONE_TOKEN,
                nonce: 0,
            };

            system_env::set_storage(
                format!("accounts.{}", system_env::signer().to_string()),
                account,
            );
        }
    } else {
        let signer = get_account(signer_id).expect("Signer not found");
        if !signed_tx.verify(signer.public_key.as_slice().try_into().unwrap()) {
            panic!("Invalid signature");
        }
    }

    let SignedTransaction {
        transaction: tx,
        signature: _,
        attachments: _,
    } = signed_tx;

    if tx.receiver_id.to_string().as_str() == SYSTEM_META_CONTRACT_ACCOUNT_ID {
        for call in tx.calls {
            match call.method.as_str() {
                "create_account" => create_account(call),
                "deploy_contract" => deploy_contract(call),
                _ => panic!("Method not found"),
            }
        }
    } else {
        for call in tx.calls {
            contract_call(call);
        }
    }
}

fn create_account(call: ContractCall) {
    let req: AccountCreationRequest = call.try_deserialize_args().unwrap();
    let None =
        system_env::get_storage::<Account>(format!("accounts.{}", req.account_id.to_string()))
    else {
        panic!("Account already exists");
    };

    let caller_id = system_env::caller();
    let caller =
        system_env::get_storage::<Account>(format!("accounts.{}", caller_id.to_string())).unwrap();

    caller
        .balance
        .checked_sub(1 * ONE_TOKEN)
        .expect("Not enough balance");

    let account = Account {
        account_id: req.account_id.clone(),
        public_key: req.public_key,
        image_id: None,
        balance: 1 * ONE_TOKEN,
        nonce: 0,
    };

    system_env::set_storage(format!("accounts.{}", req.account_id.to_string()), account);
    system_env::commit(())
}

fn account_info(context: ContractCallContext) {
    let account_id: AccountId = context.contract_call.try_deserialize_args().unwrap();

    let Some(account) =
        system_env::get_storage::<Account>(format!("accounts.{}", account_id.to_string()))
    else {
        panic!("Account does not exist");
    };

    system_env::commit(account)
}

fn deploy_contract(call: ContractCall) {
    let req: ContractDeploymentArgs = call.try_deserialize_args().unwrap();
    let contract_id = system_env::signer();
    let Some(mut account) =
        system_env::get_storage::<Account>(format!("accounts.{}", contract_id.to_string()))
    else {
        panic!("Account does not exist");
    };

    account.image_id = Some(req.image_id);
    system_env::set_storage(format!("accounts.{}", contract_id.to_string()), account);

    system_env::deploy_contract(contract_id, req.image_id);

    system_env::commit(());
}

fn contract_call(call: ContractCall) {
    let commitment = system_env::cross_contract_call_raw(
        system_env::contract(),
        call.method,
        call.gas,
        call.args,
    );

    let signer_id = system_env::signer();
    let mut signer = get_account(signer_id.clone()).expect("Signer not found");
    signer.balance = signer
        .balance
        .checked_sub(call.gas as u128)
        .expect("Not enough balance");

    system_env::set_storage(format!("accounts.{}", signer_id.to_string()), signer);

    system_env::commit(commitment);
}

fn get_account(account_id: AccountId) -> Option<Account> {
    system_env::get_storage::<Account>(format!("accounts.{}", account_id.to_string()))
}
