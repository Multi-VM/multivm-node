use std::collections::HashMap;

use borsh::{BorshDeserialize, BorshSerialize};
use multivm_primitives::{
    k256::ecdsa::SigningKey, AccountId, Attachments, Block, ContractCall, ContractCallContext,
    ContractResponse, Digest, EnvironmentContext, EvmAddress, MultiVmAccountId, SignedTransaction,
    SupportedTransaction, Transaction,
};
use multivm_runtime::{account::Account, MultivmNode};
use rand::rngs::OsRng;
use tracing::info;

pub fn install_tracing() {
    use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter};

    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| {
        // "warn,multivm_runtime=info,multivm_primitives=debug,erc20,example_token,root,fibonacci=trace,benchmarks=trace,amm=trace"
        "warn,multivm_runtime=debug,multivm_primitives=debug,erc20,example_token,root,fibonacci=trace,benchmarks=trace,amm=trace"
            .to_owned()
    });
    println!("RUST_LOG={}", filter);

    let main_layer = fmt::layer()
        .event_format(fmt::format().with_ansi(true))
        .with_filter(EnvFilter::from(filter));

    let registry = registry().with(main_layer);

    registry.init();
}

pub struct NodeHelper {
    pub node: MultivmNode,

    keys: HashMap<AccountId, SigningKey>,
}

impl NodeHelper {
    fn super_account_id() -> MultiVmAccountId {
        MultiVmAccountId::try_from("super.multivm").unwrap()
    }

    pub fn new(db_path: Option<String>) -> Self {
        let db_path = db_path.unwrap_or_else(|| {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis();
            let db_path = std::env::temp_dir()
                .join("multivm_node")
                .join(ts.to_string());
            db_path.into_os_string().into_string().unwrap()
        });

        let mut helper = Self {
            node: multivm_runtime::MultivmNode::new(db_path),
            keys: Default::default(),
        };

        let super_sk = multivm_primitives::k256::ecdsa::SigningKey::from_slice(
            &hex::decode(Self::SUPER_ACCOUNT_SK.to_string()).unwrap(),
        )
        .unwrap();

        helper
            .keys
            .insert(Self::super_account_id().into(), super_sk);

        let super_account = helper.account(&Self::super_account_id().into());
        match super_account {
            None => {
                helper.create_super_account_now();
            }
            _ => {}
        }
        helper
    }

    const SUPER_ACCOUNT_SK: &'static str =
        "4146c7e323d0ddae7baebd8e0dccbee723c9795c904d004e43a33e17adc8aa2e";

    fn create_super_account_now(&mut self) {
        let account_id = Self::super_account_id();

        let latest_block = self.node.latest_block();
        let sk = self.keys.get(&account_id.into()).unwrap();
        let address: EvmAddress = (*sk.verifying_key()).into();

        let tx = multivm_primitives::TransactionBuilder::new(
            AccountId::system_meta_contract(),
            vec![ContractCall::new(
                "init_debug_account".into(),
                &address,
                100_000_000,
                0,
            )],
            AccountId::system_meta_contract(),
            &latest_block,
        )
        .build();

        let tx = SignedTransaction {
            transaction: tx,
            signature: Default::default(),
            attachments: None,
            recovery_id: 0,
        };

        self.node.add_tx(tx.into());
        self.produce_block(true);
    }

    pub fn create_account(&mut self, multivm_account_id: &MultiVmAccountId) -> Digest {
        let mut csprng = OsRng;
        let sk = multivm_primitives::k256::ecdsa::SigningKey::random(&mut csprng);
        self.keys
            .insert(multivm_account_id.clone().into(), sk.clone());
        let latest_block = self.node.latest_block();
        let tx = create_account_tx(
            &latest_block,
            multivm_account_id.clone(),
            Self::super_account_id().into(),
            (*sk.verifying_key()).into(),
        );
        let tx_hash = tx.hash();
        let tx =
            SignedTransaction::new(tx, self.keys.get(&Self::super_account_id().into()).unwrap());

        self.node.add_tx(tx.into());

        tx_hash
    }

    pub fn create_evm_account(
        &mut self,
        multivm_account_id: &MultiVmAccountId,
        address: EvmAddress,
    ) -> EvmAddress {
        let latest_block = self.node.latest_block();

        let tx = create_account_tx(
            &latest_block,
            multivm_account_id.clone(),
            Self::super_account_id().into(),
            address.clone(),
        );
        let tx =
            SignedTransaction::new(tx, self.keys.get(&Self::super_account_id().into()).unwrap());

        self.node.add_tx(tx.into());

        address
    }

    pub fn create_contract(
        &mut self,
        multivm_contract_id: &MultiVmAccountId,
        contract_type: String,
        code: Vec<u8>,
    ) -> (Digest, Digest) {
        (
            self.create_account(multivm_contract_id),
            self.deploy_contract(multivm_contract_id, contract_type, code),
        )
    }

    pub fn deploy_contract(
        &mut self,
        multivm_contract_id: &MultiVmAccountId,
        contract_type: String,
        code: Vec<u8>,
    ) -> Digest {
        let key = self.keys.get(&multivm_contract_id.clone().into()).unwrap();
        self.deploy_contract_with_key(multivm_contract_id, contract_type, code, key.clone())
    }

    pub fn deploy_contract_with_key(
        &mut self,
        multivm_contract_id: &MultiVmAccountId,
        contract_type: String,
        code: Vec<u8>,
        key: SigningKey,
    ) -> Digest {
        let latest_block = self.node.latest_block();
        self.keys
            .insert(multivm_contract_id.clone().into(), key.clone());
        let (tx, attachs) = deploy_contract_tx(
            &latest_block,
            multivm_contract_id.clone().into(),
            contract_type,
            code,
        );
        let tx_hash = tx.hash();
        let tx = SignedTransaction::new_with_attachments(tx, &key, attachs);

        self.node.add_tx(tx.into());

        tx_hash
    }

    pub fn call_contract(
        &mut self,
        signer_id: &AccountId,
        contract_id: &AccountId,
        call: ContractCall,
    ) -> Digest {
        let latest_block = self.node.latest_block();

        let tx = multivm_primitives::TransactionBuilder::new(
            contract_id.clone(),
            vec![call],
            signer_id.clone(),
            &latest_block,
        )
        .build();

        let tx_hash = tx.hash();
        let tx = SignedTransaction::new(tx, self.keys.get(signer_id).unwrap());
        self.node.add_tx(SupportedTransaction::MultiVm(tx.into()));

        tx_hash
    }

    pub fn produce_block(&mut self, skip_proof: bool) -> Block {
        self.node.produce_block(skip_proof)
    }

    pub fn account(&self, account_id: &AccountId) -> Option<Account> {
        self.node.account_info(account_id)
    }

    pub fn view(&self, contract_id: &AccountId, call: ContractCall) -> ContractResponse {
        let response = self
            .node
            .contract_view(multivm_runtime::viewer::SupportedView::MultiVm(
                ContractCallContext {
                    contract_id: contract_id.clone(),
                    contract_call: call,
                    sender_id: AccountId::system_meta_contract(),
                    signer_id: AccountId::system_meta_contract(),
                    environment: EnvironmentContext { block_height: 0 },
                },
            ));

        response
    }
}

fn create_account_tx(
    latest_block: &Block,
    multivm_account_id: MultiVmAccountId,
    signer_id: AccountId,
    address: EvmAddress,
) -> Transaction {
    #[derive(BorshDeserialize, BorshSerialize)]
    struct AccountCreationRequest {
        pub account_id: MultiVmAccountId,
        pub address: EvmAddress,
    }

    let args = AccountCreationRequest {
        account_id: multivm_account_id,
        address,
    };

    multivm_primitives::TransactionBuilder::new(
        AccountId::system_meta_contract(),
        vec![ContractCall::new(
            "create_account".to_string(),
            &args,
            100_000_000,
            0,
        )],
        signer_id,
        &latest_block,
    )
    .build()
}

fn deploy_contract_tx(
    latest_block: &Block,
    account_id: AccountId,
    contract_type: String,
    code: Vec<u8>,
) -> (Transaction, Attachments) {
    #[derive(BorshDeserialize, BorshSerialize)]
    struct ContractDeploymentRequest {
        pub image_id: [u32; 8],
        pub contract_type: String,
    }
    let program = risc0_zkvm::Program::load_elf(&code, 0x10000000).unwrap();
    let image = risc0_zkvm::MemoryImage::new(&program, 0x400).unwrap();
    let image_id: [u32; 8] = image.compute_id().as_words().try_into().unwrap();

    let mut contracts_images = HashMap::new();
    contracts_images.insert(image_id.clone(), code);

    let args = ContractDeploymentRequest {
        image_id,
        contract_type,
    };

    let tx = multivm_primitives::TransactionBuilder::new(
        AccountId::system_meta_contract(),
        vec![ContractCall::new(
            "deploy_contract".to_string(),
            &args,
            100_000_000,
            0,
        )],
        account_id,
        &latest_block,
    )
    .build();

    let attachments = Attachments { contracts_images };

    (tx, attachments)
}
