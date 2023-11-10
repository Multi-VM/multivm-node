use std::collections::HashMap;

use borsh::{BorshDeserialize, BorshSerialize};
use eth_primitive_types::H160;
use k256::ecdsa::VerifyingKey;
use multivm_primitives::{
    k256::ecdsa::SigningKey, AccountId, Attachments, Block, ContractCall, ContractCallContext,
    Digest, EnvironmentContext, EvmAddress, MultiVmAccountId, SignedTransaction,
    SupportedTransaction, Transaction,
};
use multivm_runtime::MultivmNode;
use rand::rngs::OsRng;

pub fn install_tracing() {
    use tracing_subscriber::{fmt, prelude::*, registry, EnvFilter};

    let filter = std::env::var("RUST_LOG").unwrap_or_else(|_| {
        "warn,multivm_runtime=info,multivm_primitives=debug,erc20,example_token,root,fibonacci=trace,benchmarks=trace"
            .to_owned()
    });
    println!("RUST_LOG={}", filter);

    let main_layer = fmt::layer()
        .event_format(fmt::format().with_ansi(true))
        .with_filter(EnvFilter::from(filter));

    let registry = registry().with(main_layer);

    registry.init();
}

pub fn init_temp_node() -> MultivmNode {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let db_path = std::env::temp_dir()
        .join("multivm_node")
        .join(ts.to_string());
    let mut node = multivm_runtime::MultivmNode::new(String::from(db_path.to_str().unwrap()));

    node.init_genesis();

    node
}

pub struct NodeHelper {
    pub node: MultivmNode,

    keys: HashMap<AccountId, SigningKey>,
}

impl NodeHelper {
    fn super_account_id() -> MultiVmAccountId {
        MultiVmAccountId::try_from("super.multivm").unwrap()
    }

    pub fn new_temp() -> Self {
        let mut helper = Self {
            node: init_temp_node(),
            keys: Default::default(),
        };

        helper.create_super_account_now();

        helper
    }

    fn create_super_account_now(&mut self) {
        let mut csprng = OsRng;
        let sk = multivm_primitives::k256::ecdsa::SigningKey::random(&mut csprng);
        let account_id = Self::super_account_id();
        self.keys.insert(account_id.clone().into(), sk.clone());

        let latest_block = self.node.latest_block();

        let tx = multivm_primitives::TransactionBuilder::new(
            AccountId::system_meta_contract(),
            vec![ContractCall::new(
                "init_debug_account".into(),
                &sk.verifying_key().to_sec1_bytes().to_vec(),
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
            sk.verifying_key(),
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
        vk: VerifyingKey,
    ) -> H160 {
        let latest_block = self.node.latest_block();
        let tx = create_account_tx(
            &latest_block,
            multivm_account_id.clone(),
            Self::super_account_id().into(),
            &vk,
        );
        let tx =
            SignedTransaction::new(tx, self.keys.get(&Self::super_account_id().into()).unwrap());

        self.node.add_tx(tx.into());

        let point = vk.to_encoded_point(false);
        let hash = ethers_core::utils::keccak256(&point.as_bytes()[1..]);
        eth_primitive_types::H160::from_slice(&hash[12..]).into()
    }

    pub fn create_contract(
        &mut self,
        multivm_contract_id: &MultiVmAccountId,
        code: Vec<u8>,
    ) -> (Digest, Digest) {
        (
            self.create_account(multivm_contract_id),
            self.deploy_contract(multivm_contract_id, code),
        )
    }

    pub fn deploy_contract(
        &mut self,
        multivm_contract_id: &MultiVmAccountId,
        code: Vec<u8>,
    ) -> Digest {
        let key = self.keys.get(&multivm_contract_id.clone().into()).unwrap();
        let latest_block = self.node.latest_block();
        let (tx, attachs) =
            deploy_contract_tx(&latest_block, multivm_contract_id.clone().into(), code);
        let tx_hash = tx.hash();
        let tx = SignedTransaction::new_with_attachments(tx, key, attachs);

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

    pub fn account(&self, account_id: &AccountId) -> Account {
        let bytes = self
            .node
            .view(multivm_runtime::viewer::SupportedView::MultiVm(
                ContractCallContext {
                    contract_id: AccountId::system_meta_contract(),
                    contract_call: ContractCall::new(
                        "account_info".to_string(),
                        account_id,
                        100_000_000,
                        0,
                    ),
                    sender_id: AccountId::system_meta_contract(),
                    signer_id: AccountId::system_meta_contract(),
                    environment: EnvironmentContext { block_height: 0 },
                },
            ));

        borsh::from_slice(&bytes).unwrap()
    }
}

fn create_account_tx(
    latest_block: &Block,
    multivm_account_id: MultiVmAccountId,
    signer_id: AccountId,
    pk: &multivm_primitives::k256::ecdsa::VerifyingKey,
) -> Transaction {
    #[derive(BorshDeserialize, BorshSerialize)]
    struct AccountCreationRequest {
        pub account_id: MultiVmAccountId,
        pub public_key: Vec<u8>,
    }

    let args = AccountCreationRequest {
        account_id: multivm_account_id,
        public_key: pk.to_sec1_bytes().to_vec(),
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
    code: Vec<u8>,
) -> (Transaction, Attachments) {
    #[derive(BorshDeserialize, BorshSerialize)]
    struct ContractDeploymentRequest {
        pub image_id: [u32; 8],
    }
    let program = risc0_zkvm::Program::load_elf(&code, 0x10000000).unwrap();
    let image = risc0_zkvm::MemoryImage::new(&program, 0x400).unwrap();
    let image_id: [u32; 8] = image.compute_id().as_words().try_into().unwrap();

    let mut contracts_images = HashMap::new();
    contracts_images.insert(image_id.clone(), code);

    let args = ContractDeploymentRequest { image_id };

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
