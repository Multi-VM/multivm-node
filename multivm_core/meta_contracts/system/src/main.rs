#![no_main]

use core::panic;
use std::io::Read;

use account_management::{update_account, MultiVmExecutable, SolanaExecutable};
use borsh::{BorshDeserialize, BorshSerialize};
use ethers_core::types::NameOrAddress;
use multivm_primitives::{
    AccountId, ContractCall, ContractCallContext, EnvironmentContext, EthereumTransactionRequest,
    EvmAddress, MultiVmAccountId, SignedTransaction, SupportedTransaction,
};

use crate::account_management::Executable;

mod evm;
mod system_env;

const TOKEN_DECIMALS: u32 = 18;
const ONE_TOKEN: u128 = 10u128.pow(TOKEN_DECIMALS);

#[derive(BorshDeserialize, BorshSerialize, Debug)]
struct AccountCreationRequest {
    pub account_id: MultiVmAccountId,
    pub address: EvmAddress,
}

#[derive(BorshDeserialize, BorshSerialize)]
struct ContractDeploymentArgs {
    pub image_id: [u32; 8],
    pub contract_type: String, // TODO: use enum
}

#[derive(BorshDeserialize, BorshSerialize)]
struct EvmCall {
    pub from: Option<[u8; 20]>,
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
    Call(ContractCallContext),
    EvmCall(ContractCallContext),
}
risc0_zkvm::entry!(entrypoint);

fn entrypoint() {
    let mut bytes = Vec::<u8>::new();
    risc0_zkvm::guest::env::stdin()
        .read_to_end(&mut bytes)
        .unwrap();

    let action: Action = BorshDeserialize::try_from_slice(&mut bytes).expect("Corrupted action");

    match action {
        Action::ExecuteTransaction(tx, environment) => match tx {
            SupportedTransaction::MultiVm(tx) => process_transaction(tx, environment),
            SupportedTransaction::Evm(tx) => process_ethereum_transaction(tx, environment),
            SupportedTransaction::Solana(tx) => process_solana_transaction(tx, environment),
        },
        Action::View(v, environment) => match v {
            SupportedView::MultiVm(context) => view(context),
            SupportedView::Evm(call) => evm_view_call(call, environment),
        },
        Action::Call(ctx) => {
            system_env::setup_env(&ctx);
            process_call(ctx.clone().contract_id, ctx.clone().contract_call, ctx)
        }
        Action::EvmCall(ctx) => {
            system_env::setup_env(&ctx);
            evm_call(ctx)
        }
    };
}

fn process_ethereum_transaction(tx: EthereumTransactionRequest, environment: EnvironmentContext) {
    let (tx, sign) = tx.decode();
    if !sign.verify(tx.sighash(), tx.from.unwrap()).is_ok() {
        panic!("Invalid signature");
    }

    let contract_call = ContractCall {
        method: "".to_string(),
        args: vec![],
        gas: 300_000,
        deposit: tx.value.unwrap_or_default().try_into().unwrap(),
    };
    let ctx = ContractCallContext {
        contract_id: AccountId::system_meta_contract(),
        contract_call,
        sender_id: AccountId::system_meta_contract(),

        signer_id: AccountId::system_meta_contract(),
        environment,
    };
    system_env::setup_env(&ctx);

    let caller = account_management::account(
        &EvmAddress::from(tx.from.expect("no 'from', probably tx is not signed")).into(),
    )
    .expect(format!("Caller not found: {:#?}", tx.from).as_str()); // TODO: handle error

    match tx.processing_flow() {
        EthereumTxFlow::Deploy(bytecode) => evm::deploy_evm_contract(caller, bytecode),
        EthereumTxFlow::Call(contract_id, data) => {
            let contract = account_management::account(&contract_id.clone().into()).unwrap();
            match contract.executable {
                Some(Executable::Evm()) => evm::call_contract(
                    caller.evm_address,
                    contract_id,
                    data,
                    ctx.contract_call.deposit,
                    true,
                ),
                Some(Executable::MultiVm(_)) => {
                    let Some(multivm_contract_id) = contract.multivm_account_id else {
                        panic!("Contract is MultiVM executable but has no multivm account");
                    };
                    process_call(
                        multivm_contract_id.into(),
                        borsh::from_slice(&data)
                            .expect("multivm tx data was incorrectly serialized"),
                        ctx,
                    )
                }
                Some(Executable::Solana(_)) => {
                    let Some(solana_address) = contract.solana_address else {
                        panic!("Contract is Solana executable but has no solana account");
                    };
                    process_call(
                        solana_address.into(),
                        borsh::from_slice(&data)
                            .expect("multivm tx data was incorrecly serialized"),
                        ctx,
                    )
                }
                _ => panic!("Executable not supported"),
            }
        }
    };
}

enum EthereumTxFlow {
    Deploy(Vec<u8>),
    Call(EvmAddress, Vec<u8>),
}

trait TransactionFlow<T> {
    fn processing_flow(self) -> T;
}

impl TransactionFlow<EthereumTxFlow>
    for ethers_core::types::transaction::request::TransactionRequest
{
    fn processing_flow(self) -> EthereumTxFlow {
        let receiver_id: EvmAddress = match self.to {
            None => return EthereumTxFlow::Deploy(self.data.unwrap().to_vec()),
            Some(NameOrAddress::Address(address)) => address.into(),
            _ => panic!("Not supported"),
        };

        let data = match self.data {
            None => vec![],
            Some(data) => data.to_vec(),
        };

        EthereumTxFlow::Call(receiver_id, data)
    }
}

fn process_solana_transaction(_bytes: Vec<u8>, _environment: EnvironmentContext) {
    unimplemented!();
}

fn view(context: ContractCallContext) {
    system_env::setup_env(&context);

    match context.contract_call.method.as_str() {
        "account_info" => account_info(context),
        _ => panic!("Method not found"),
    }
}

fn evm_view_call(call: EvmCall, environment: EnvironmentContext) {
    let contract_call = ContractCall {
        method: "".to_string(),
        args: vec![],
        gas: 300_000,
        deposit: 0,
    };
    let ctx = ContractCallContext {
        contract_id: AccountId::system_meta_contract(),
        contract_call,
        sender_id: AccountId::system_meta_contract(),
        signer_id: AccountId::system_meta_contract(),
        environment,
    };
    system_env::setup_env(&ctx);

    let caller_address = call
        .from
        .map(|from| eth_primitive_types::H160::from(from))
        .unwrap_or_default();
    let contract_address = eth_primitive_types::H160::from_slice(&call.to).into();
    evm::call_contract(
        caller_address.into(),
        contract_address,
        call.input,
        0,
        false,
    )
}

fn evm_call(ctx: ContractCallContext) {
    let caller = account_management::account(&system_env::caller())
        .expect(&format!("Caller not found: {}", system_env::caller())); // TODO: handle error
    let contract =
        account_management::account(&system_env::contract()).expect("Contract not found"); // TODO: handle error

    evm::call_contract(
        caller.evm_address,
        contract.evm_address,
        ctx.contract_call.args,
        ctx.contract_call.deposit,
        true,
    )
}

// TODO: remove this
fn init_debug_account(address: EvmAddress) {
    let mut account = account_management::Account::try_create(
        Some(MultiVmAccountId::try_from("super.multivm").unwrap()),
        address,
    );
    account.balance = 1_000_000_000_000 * ONE_TOKEN;
    update_account(account);
    system_env::commit(());
}

fn process_transaction(signed_tx: SignedTransaction, environment: EnvironmentContext) {
    let ctx = signed_tx.transaction.context(0, environment.clone());

    system_env::setup_env(&ctx);

    // Skip signature verification for debug purposes
    // TODO: remove
    if ctx.environment.block_height > 1 {
        let signer_id = system_env::signer();

        let signer = account_management::account(&signer_id)
            .expect(&format!("Signer not found: {}", signer_id)); // TODO: handle error

        if !signed_tx.verify(signer.evm_address) {
            panic!("Invalid signature"); // TODO: handle error
        }
    }

    let SignedTransaction {
        transaction: tx,
        signature: _,
        recovery_id: _,
        attachments: _,
    } = signed_tx;

    for call in tx.calls {
        process_call(tx.receiver_id.clone(), call, ctx.clone());
    }
}

fn process_call(contract_id: AccountId, call: ContractCall, ctx: ContractCallContext) {
    if contract_id == AccountId::system_meta_contract() {
        match call.method.as_str() {
            "create_account" => create_account(call),
            "deploy_contract" => deploy_multivm_contract(call),
            "init_debug_account" => init_debug_account(call.try_deserialize_args().unwrap()),
            "account_info" => account_info(ctx),
            "transfer" => transfer(ctx),
            _ => panic!("Method not found"),
        }
    } else {
        contract_call(contract_id, call);
    }
}

fn create_account(call: ContractCall) {
    let req: AccountCreationRequest = call.try_deserialize_args().unwrap();

    let mut account = account_management::Account::try_create(Some(req.account_id), req.address);
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
    let account = account_management::account(&account_id);
    system_env::commit(account)
}

fn transfer(context: ContractCallContext) {
    let (receiver, amount): (AccountId, u128) =
        context.contract_call.try_deserialize_args().unwrap();
    let sender = account_management::account(&context.sender_id).unwrap();
    account_management::transfer(sender, receiver, amount);
    system_env::commit(())
}

fn deploy_multivm_contract(call: ContractCall) {
    let req: ContractDeploymentArgs = call.try_deserialize_args().unwrap();
    let mut account =
        account_management::account(&system_env::signer()).expect("Account not found"); // TODO: handle error

    system_env::deploy_contract(system_env::signer(), req.image_id);
    account.executable = match req.contract_type.as_str() {
        "mvm" => Some(
            MultiVmExecutable {
                image_id: req.image_id,
            }
            .into(),
        ),
        "svm" => Some(
            SolanaExecutable {
                image_id: req.image_id,
            }
            .into(),
        ),
        _ => {
            panic!("Unknown contract type")
        }
    };
    update_account(account);
    system_env::commit(());
}

fn contract_call(contract_id: AccountId, call: ContractCall) {
    let commitment =
        system_env::cross_contract_call_raw(contract_id, call.method, call.gas, call.args);

    let signer_id = system_env::signer();
    // if signer == system_meta then its multivm call in evm wrapper probably
    if signer_id != AccountId::system_meta_contract() {
        let mut signer = account_management::account(&signer_id).expect("Signer account not found"); // TODO: handle error
        signer.balance = signer
            .balance
            .checked_sub(call.gas as u128)
            .expect(&format!(
                "Not enough balance for {} (balance {}, required {})",
                signer_id, signer.balance, call.gas
            ));

        account_management::update_account(signer);
    }

    system_env::commit(commitment);
}

mod account_management {
    use borsh::{BorshDeserialize, BorshSerialize};
    use multivm_primitives::{AccountId, EvmAddress, MultiVmAccountId, SolanaAddress};

    use crate::system_env;

    #[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
    pub enum Executable {
        Evm(),
        MultiVm(MultiVmExecutable),
        Solana(SolanaExecutable),
    }

    impl From<MultiVmExecutable> for Executable {
        fn from(executable: MultiVmExecutable) -> Self {
            Self::MultiVm(executable)
        }
    }

    impl From<SolanaExecutable> for Executable {
        fn from(executable: SolanaExecutable) -> Self {
            Self::Solana(executable)
        }
    }

    #[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
    pub struct MultiVmExecutable {
        pub image_id: [u32; 8],
    }

    #[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
    pub struct SolanaExecutable {
        pub image_id: [u32; 8],
    }

    #[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
    pub struct Account {
        internal_id: u128,
        pub evm_address: EvmAddress,
        pub multivm_account_id: Option<MultiVmAccountId>,
        pub solana_address: Option<SolanaAddress>,
        pub executable: Option<Executable>,
        pub balance: u128,
        pub nonce: u64,
    }

    impl Account {
        pub fn try_create(
            multivm_account_id: Option<MultiVmAccountId>,
            address: EvmAddress,
        ) -> Self {
            let multivm_exists = multivm_account_id
                .clone()
                .map(|multivm_account_id| account_exists(&multivm_account_id.into()))
                .unwrap_or_default();

            let evm_exists = account_exists(&address.clone().into());

            if multivm_exists || evm_exists {
                panic!("Account alias already exists"); // TODO: handle error
            }

            let mut temp_solana_address = [0u8; 32];
            temp_solana_address[0..20].copy_from_slice(&address.to_bytes());

            let account = Self {
                internal_id: increment_account_counter(),
                evm_address: address,
                multivm_account_id,
                solana_address: Some(temp_solana_address.into()),
                executable: None,
                balance: 0,
                nonce: 0,
            };

            println!(
                "account created: MultiVM: ({:?}), EVM: ({:?}), Solana: ({:?})",
                account
                    .multivm_account_id
                    .clone()
                    .map(|id| { id.to_string() }),
                account.evm_address.clone().to_string(),
                account.solana_address.clone().unwrap().to_string(),
            );

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
            AccountId::Solana(solana_address) => {
                account_internal_id_by_solana_alias(solana_address)
            }
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
            AccountId::Solana(solana_address) => {
                account_internal_id_by_solana_alias(solana_address)
            }
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

        account.solana_address.map(|solana_address| {
            if account_exists(&solana_address.clone().into()) {
                panic!("Account alias already exists"); // TODO: handle error
            }
            system_env::set_storage(
                format!("accounts_aliases.solana.{}", solana_address),
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
        let address = evm_address.to_string().to_lowercase();
        system_env::get_storage(format!("accounts_aliases.evm.{}", address))
    }

    fn account_internal_id_by_solana_alias(solana_address: SolanaAddress) -> Option<u128> {
        let address = solana_address.to_string();
        system_env::get_storage(format!("accounts_aliases.solana.{}", address))
    }

    /// Updates existing account in the system
    pub fn update_account(account: Account) {
        if !account_exists_by_internal_id(account.internal_id) {
            panic!("Account update requires existing account");
        }
        system_env::set_storage(format!("accounts.{}", account.internal_id), account);
    }

    pub fn transfer(mut sender: Account, receiver_id: AccountId, amount: u128) {
        sender.balance -= amount;
        update_account(sender);

        let mut receiver = account(&receiver_id).expect("Receiver not found");
        receiver.balance += amount;
        update_account(receiver);
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
