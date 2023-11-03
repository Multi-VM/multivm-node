#![no_main]

use core::panic;
use std::str::FromStr;

use account_management::update_account;
use borsh::{BorshDeserialize, BorshSerialize};
use eth_primitive_types::H160;
use multivm_primitives::{
    AccountId, ContractCall, ContractCallContext, EnvironmentContext, EvmAddress, MultiVmAccountId,
    SignedTransaction, SupportedTransaction,
};

mod evm;
mod system_env;

const TOKEN_DECIMALS: u32 = 18;
const ONE_TOKEN: u128 = 10u128.pow(TOKEN_DECIMALS);

#[derive(BorshDeserialize, BorshSerialize, Debug)]
struct AccountCreationRequest {
    pub account_id: MultiVmAccountId,
    pub public_key: Vec<u8>,
}

#[derive(BorshDeserialize, BorshSerialize)]
struct ContractDeploymentArgs {
    pub image_id: [u32; 8],
}

#[derive(BorshDeserialize, BorshSerialize)]
struct EvmCall {
    pub from: [u8; 20],
    pub to: [u8; 20],
    pub input: Vec<u8>,
}

#[derive(BorshDeserialize, BorshSerialize)]
enum SupportedView {
    MultiVm(ContractCallContext),
    Evm(EvmCall),
}

#[derive(BorshDeserialize, BorshSerialize)]
enum Action {
    ExecuteTransaction(SupportedTransaction, EnvironmentContext),
    View(SupportedView, EnvironmentContext),
}
risc0_zkvm::entry!(entrypoint);

fn entrypoint() {
    let mut bytes: Vec<u8> = risc0_zkvm::guest::env::read();

    let action: Action = BorshDeserialize::try_from_slice(&mut bytes).expect("Corrupted action");

    match action {
        Action::ExecuteTransaction(tx, environment) => match tx {
            SupportedTransaction::MultiVm(tx) => process_transaction(tx, environment),
            SupportedTransaction::Evm(tx) => process_ethereum_transaction(tx, environment),
        },
        Action::View(v, environment) => match v {
            SupportedView::MultiVm(context) => view(context),
            SupportedView::Evm(call) => evm_call(call, environment),
        },
    };
}

fn process_ethereum_transaction(bytes: Vec<u8>, environment: EnvironmentContext) {
    let rlp = ethers_core::utils::rlp::Rlp::new(&bytes);
    let (tx, sign) = ethers_core::types::TransactionRequest::decode_signed_rlp(&rlp).unwrap();
    if !sign
        .verify(tx.rlp_unsigned().to_vec(), tx.from.unwrap())
        .is_ok()
    {
        panic!("Invalid signature");
    }

    let contract_call = ContractCall { method: "".to_string(), args: vec![], gas: 300_000, deposit: 0 };
    let ctx = ContractCallContext {
        contract_id: AccountId::system_meta_contract(),
        contract_call,
        sender_id: AccountId::system_meta_contract(),
        signer_id: AccountId::system_meta_contract(),
        environment,
    };
    system_env::setup_env(&ctx);

    let caller = account_management::account(&EvmAddress::from(tx.from.unwrap()).into())
        .expect("Caller not found"); // TODO: handle error

    if tx.to.is_none() {
        evm::deploy_evm_contract(caller, tx.data.expect("No data").to_vec());
    } else {
        evm::call_contract(
            caller,
            EvmAddress::from(tx.to.unwrap().as_address().unwrap().clone()),
            tx.data.expect("No data").to_vec(),
            true,
        );
    }
}

fn view(context: ContractCallContext) {
    system_env::setup_env(&context);

    match context.contract_call.method.as_str() {
        "account_info" => account_info(context),
        _ => panic!("Method not found"),
    }
}

fn evm_call(call: EvmCall, environment: EnvironmentContext) {
    let contract_call = ContractCall { method: "".to_string(), args: vec![], gas: 300_000, deposit: 0 };
    let ctx = ContractCallContext {
        contract_id: AccountId::system_meta_contract(),
        contract_call,
        sender_id: AccountId::system_meta_contract(),
        signer_id: AccountId::system_meta_contract(),
        environment,
    };
    system_env::setup_env(&ctx);

    let caller = account_management::account(
        &EvmAddress::from(eth_primitive_types::H160::from_slice(&call.from)).into(),
    )
    .expect("Caller not found"); // TODO: handle error

    let contract_address = eth_primitive_types::H160::from_slice(&call.to).into();
    evm::call_contract(caller, contract_address, call.input, false)
}

// TODO: remove this
fn init_debug_account(public_key: Vec<u8>) {
    let mut account = account_management::Account::try_create(
        Some(MultiVmAccountId::try_from("super.multivm").unwrap()),
        public_key,
    );
    account.balance = 1_000_000_000_000 * ONE_TOKEN;
    update_account(account);
    system_env::commit(());
}

fn process_transaction(signed_tx: SignedTransaction, environment: EnvironmentContext) {
    let ctx = signed_tx.transaction.context(0, environment);

    system_env::setup_env(&ctx);

    // Skip signature verification for debug purposes
    // TODO: remove
    if ctx.environment.block_height > 1 {
        let signer_id = system_env::signer();

        let signer = account_management::account(&signer_id).expect("Signer not found"); // TODO: handle error

        // TODO: handle None in public key
        if !signed_tx.verify(signer.public_key.unwrap().as_slice().try_into().unwrap()) {
            panic!("Invalid signature"); // TODO: handle error
        }
    }

    let SignedTransaction {
        transaction: tx,
        signature: _,
        attachments: _,
    } = signed_tx;

    if tx.receiver_id == AccountId::system_meta_contract() {
        for call in tx.calls {
            match call.method.as_str() {
                "create_account" => create_account(call),
                "deploy_contract" => deploy_contract(call),
                "init_debug_account" => init_debug_account(call.try_deserialize_args().unwrap()),
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

    let mut account = account_management::Account::try_create(Some(req.account_id), req.public_key);
    let caller = account_management::account(&system_env::caller()).unwrap();

    caller
        .balance
        .checked_sub(1_000 * ONE_TOKEN)
        .expect("Not enough balance"); // TODO: handle error

    account.balance = account.balance.checked_add(1_000 * ONE_TOKEN).unwrap();

    update_account(account);
    update_account(caller);

    system_env::commit(())
}

fn account_info(context: ContractCallContext) {
    let account_id: AccountId = context.contract_call.try_deserialize_args().unwrap();
    let account = account_management::account(&account_id).expect("Account not found"); // TODO: handle error
    system_env::commit(account)
}

fn deploy_contract(call: ContractCall) {
    let req: ContractDeploymentArgs = call.try_deserialize_args().unwrap();
    let mut account =
        account_management::account(&system_env::signer()).expect("Account not found"); // TODO: handle error

    system_env::deploy_contract(system_env::signer(), req.image_id);
    account.image_id = Some(req.image_id);
    update_account(account);
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
    let mut signer = account_management::account(&signer_id).expect("Signer account not found"); // TODO: handle error
    signer.balance = signer
        .balance
        .checked_sub(call.gas as u128)
        .expect(&format!(
            "Not enough balance for {} (balance {}, required {})",
            signer_id, signer.balance, call.gas
        ));

    account_management::update_account(signer);

    system_env::commit(commitment);
}

mod account_management {
    use borsh::{BorshDeserialize, BorshSerialize};
    use ethers_core::k256::elliptic_curve::sec1::ToEncodedPoint;
    use multivm_primitives::{AccountId, EvmAddress, MultiVmAccountId};

    use crate::system_env;

    #[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
    pub struct Account {
        internal_id: u128,
        pub evm_address: EvmAddress,
        pub multivm_account_id: Option<MultiVmAccountId>,
        pub public_key: Option<Vec<u8>>,
        pub image_id: Option<[u32; 8]>,
        pub balance: u128,
        pub nonce: u64,
    }

    impl Account {
        pub fn try_create(
            multivm_account_id: Option<MultiVmAccountId>,
            public_key: Vec<u8>,
        ) -> Self {
            let evm_address: EvmAddress = {
                let public_key =
                    k256::PublicKey::from_sec1_bytes(&public_key).expect("Invalid public key"); // TODO: handle error

                let point = public_key.to_encoded_point(false);
                let hash = ethers_core::utils::keccak256(&point.as_bytes()[1..]);
                eth_primitive_types::H160::from_slice(&hash[12..]).into()
            };

            let multivm_exists = multivm_account_id
                .clone()
                .map(|multivm_account_id| account_exists(&multivm_account_id.into()))
                .unwrap_or_default();

            let evm_exists = account_exists(&evm_address.clone().into());

            if multivm_exists || evm_exists {
                panic!("Account alias already exists"); // TODO: handle error
            }

            let account = Self {
                internal_id: increment_account_counter(),
                public_key: Some(public_key),
                evm_address,
                multivm_account_id,
                image_id: None,
                balance: 0,
                nonce: 0,
            };

            register_account(account.clone());
            account
        }

        pub fn try_create_empty_evm(evm_address: EvmAddress) -> Self {
            if account_exists(&evm_address.clone().into()) {
                panic!("Account alias already exists"); // TODO: handle error
            }

            let account = Self {
                internal_id: increment_account_counter(),
                public_key: None,
                evm_address,
                multivm_account_id: None,
                image_id: None,
                balance: 0,
                nonce: 0,
            };

            register_account(account.clone());
            account
        }
    }

    /// Returns account by alias
    pub fn account(account_id: &AccountId) -> Option<Account> {
        let id: Option<u128> = match account_id.clone() {
            AccountId::MultiVm(multivm_account_id) => {
                account_internal_id_by_multivm_alias(multivm_account_id)
            }
            AccountId::Evm(evm_address) => account_internal_id_by_evm_alias(evm_address),
        };

        id.map(|id| account_by_internal_id(id).expect("Alias points to non-existing account"))
    }

    /// Returns account by internal id
    fn account_by_internal_id(id: u128) -> Option<Account> {
        system_env::get_storage::<Account>(format!("accounts.{}", id))
    }

    pub fn increment_account_counter() -> u128 {
        let new_count: u128 = system_env::get_storage("account_counter".into()).unwrap_or(0) + 1;
        system_env::set_storage("account_counter".into(), new_count);
        new_count
    }

    /// Returns true if account exists
    pub fn account_exists(account_id: &AccountId) -> bool {
        match account_id.clone() {
            AccountId::MultiVm(multivm_account_id) => {
                account_internal_id_by_multivm_alias(multivm_account_id)
            }
            AccountId::Evm(evm_address) => account_internal_id_by_evm_alias(evm_address),
        }
        .is_some()
    }

    /// Returns true if account exists
    fn account_exists_by_internal_id(id: u128) -> bool {
        account_by_internal_id(id).is_some()
    }

    /// Registers account in the system, with alias mappings
    pub fn register_account(account: Account) {
        system_env::set_storage(format!("accounts.{}", account.internal_id), account.clone());

        account.multivm_account_id.map(|multivm_account_id| {
            if account_exists(&multivm_account_id.clone().into()) {
                panic!("Account alias already exists"); // TODO: handle error
            }
            system_env::set_storage(
                format!("accounts_aliases.multivm.{}", multivm_account_id),
                account.internal_id,
            );
        });

        {
            if account_exists(&account.evm_address.clone().into()) {
                panic!("Account alias already exists"); // TODO: handle error
            }

            system_env::set_storage(
                format!("accounts_aliases.evm.{}", account.evm_address),
                account.internal_id,
            );
        }
    }

    fn account_internal_id_by_multivm_alias(multivm_account_id: MultiVmAccountId) -> Option<u128> {
        system_env::get_storage(format!("accounts_aliases.multivm.{}", multivm_account_id))
    }

    fn account_internal_id_by_evm_alias(evm_address: EvmAddress) -> Option<u128> {
        system_env::get_storage(format!("accounts_aliases.evm.{}", evm_address))
    }

    /// Updates existing account in the system
    pub fn update_account(account: Account) {
        if !account_exists_by_internal_id(account.internal_id) {
            panic!("Account update requires existing account");
        }
        system_env::set_storage(format!("accounts.{}", account.internal_id), account);
    }

    pub fn account_storage<T: BorshDeserialize>(account_id: &AccountId, key: String) -> Option<T> {
        let account = account(account_id)?;
        system_env::get_storage(format!("accounts.{}.{}", account.internal_id, key))
    }

    pub fn update_account_storage<T: BorshSerialize>(
        account_id: &AccountId,
        key: String,
        value: T,
    ) {
        let account = account(account_id).expect("Account not found"); // TODO: handle error
        system_env::set_storage(format!("accounts.{}.{}", account.internal_id, key), value);
    }
}
