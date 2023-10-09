use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use borsh::BorshDeserialize;
use eth_primitive_types::H160;
use ethers::{core::types::TransactionRequest, utils::rlp::Rlp};
use ethers_core::k256::ecdsa::VerifyingKey;
use jsonrpsee::server::Server;
use jsonrpsee::RpcModule;
use lazy_static::lazy_static;
use multivm_primitives::{MultiVmAccountId, SupportedTransaction, AccountId};
use multivm_runtime::viewer::{EvmCall, SupportedView};
use playgrounds::NodeHelper;
use serde_json::json;
use tracing::info;

use crate::utils::{EthBlockOutput, EthTransaction, From0x, To0x, EthTransactionReceipt};

static CHAIN_ID: u64 = 1044942;

#[derive(Clone)]
pub struct MultivmServer {
    helper: Arc<Mutex<NodeHelper>>,
}

impl MultivmServer {
    pub fn new() -> Self {
        Self {
            helper: Arc::new(Mutex::new(NodeHelper::new_temp())),
        }
    }
    pub async fn start(&'static mut self) -> anyhow::Result<()> {
        let server = Server::builder().build("127.0.0.1:80").await?;
        let mut module = RpcModule::new(());

        module.register_method("eth_chainId", |_, _| {
            info!("eth_chainId: {}", CHAIN_ID.to_0x());
            CHAIN_ID.to_0x()
        })?;
        module.register_method("eth_blockNumber", |_, _| {
            let helper = self.helper.lock().unwrap();
            let height = format!("0x{:x}", helper.node.latest_block().height);
            info!("eth_blockNumber: {}", height);
            height
        })?;
        module.register_method("eth_getBalance", |params, _| {
            info!("eth_getBalance: {:#?}", params);
            let helper = self.helper.lock().unwrap();
            let address: H160 = params.sequence().next::<String>().unwrap().from_0x();
            let account = helper.account(&AccountId::Evm(address.into()));
            info!("returned: {}, hex: {}", account.balance, account.balance.to_0x());
            account.balance.to_0x()
        })?;
        module.register_method("eth_getBlockByNumber", |params, _| {
            info!("eth_getBlockByNumber: {:#?}", params.sequence());
            let helper = self.helper.lock().unwrap();
            let block_request = params.sequence().next::<String>().unwrap();
            let height = if block_request == "latest" {
                helper.node.latest_block().height
            } else {
                block_request.from_0x()
            };
            let block = helper
                .node
                .block_by_height(height)
                .expect(format!("No block at height {}", height).as_str());
            let output = EthBlockOutput::from(&block);
            info!("returned: {:#?}", output);
            json!(output)
        })?;
        module.register_method("eth_getBlockByHash", |params, _| {
            info!("eth_getBlockByHash: {:#?}", params.sequence());
            let helper = self.helper.lock().unwrap();
            let block_request = params.sequence().next::<String>().unwrap();
            let height = if block_request == "latest" {
                helper.node.latest_block().height
            } else {
                block_request.from_0x()
            };
            let block = helper.node.latest_block();
            let output = EthBlockOutput::from(&block);
            info!("returned: {:#?}", output);
            json!(output)
        })?;
        module.register_method("eth_getTransactionReceipt", |params, _| {
            let hash = params.sequence().next::<String>().unwrap();
            info!("eth_getTransactionReceipt: \n\r{:#?}", hash);
            let helper = self.helper.lock().unwrap();
            for height in 1..=helper.node.latest_block().height {
                let block = helper.node.block_by_height(height).unwrap();
                for tx in block.txs.clone() {
                    if tx.hash().to_0x() == hash {
                        let receipt = EthTransactionReceipt::from(&tx, hash, &block);
                        info!("=== returned {:#?}", receipt);
                        return json!(receipt);
                    }
                }
            }
            info!("=== returned empty receipt {{}}");
            return json!([]);
        })?;

        module.register_method("eth_sendTransaction", move |_params, _| {
            info!("eth_sendTransaction");

            // let input = params.one::<EthTransactionInput>().unwrap();
            // let contract = AccountId::from(input.to.to_string());
            // let signer = AccountId::from(input.from.to_string());
            // let mut args = input.data.split(":");
            // let method_name = args.next().unwrap();

            // let mut node = Self::node_instance();

            // match method_name {
            //     "transfer" => {
            //         let to = AccountId::from(args.next().unwrap().to_string());
            //         let amount = u128::from_str_radix(args.next().unwrap(), 10).unwrap();

            //         let (_, hash) = node.call(tx(
            //             &node.latest_block(),
            //             &contract,
            //             method_name.into(),
            //             (to, amount),
            //             &signer,
            //         ));

            //         return hash.to_0x();
            //     }
            //     "balance_of" => {
            //         let account_id = AccountId::from(args.next().unwrap().to_string());

            //         let (block, hash) = node.call(tx(
            //             &node.latest_block(),
            //             &contract,
            //             method_name.into(),
            //             account_id,
            //             &signer,
            //         ));

            //         let output: u128 = block.output(hash);
            //         info!(output);

            //         return format!("{}", output).to_string();
            //     }
            //     _ => panic!("Unknown method"),
            // };
        })?;

        module.register_method("net_version", |_, _| {
            info!("net_version");
            "1"
        })?;
        module.register_method("eth_gasPrice", |_, _| {
            info!("eth_gasPrice");
            "0x1dfd14000"
        })?;
        module.register_method("eth_getCode", |params, _| {
            info!("eth_getCode: {:#?}", params);
            "0x1dfd14000"
        })?;
        module.register_method("eth_estimateGas", |params, _| {
            info!("eth_estimateGas");
            "0x5208"
        })?;
        module.register_method("eth_getTransactionCount", |params, _| {
            info!("eth_getTransactionCount {:#?}", params.sequence());

            let address = params.sequence().next::<String>().unwrap();
            let mut tx_count: usize = 0;

            let helper = self.helper.lock().unwrap();
            for height in 1..=helper.node.latest_block().height {
                let block = helper.node.block_by_height(height).unwrap();
                for tx in block.txs {
                    match tx {
                        SupportedTransaction::MultiVm(_) => {},
                        SupportedTransaction::Evm(_) => {
                            tx_count += 1;
                        }
                    }
                }
            }

            info!("returned count {}", tx_count.to_0x());
            tx_count.to_0x()
        })?;
        module.register_method("eth_getTransactionByHash", |params, _| {
            info!("eth_getTransactionByHash {:#?}", params.sequence());

            let hash: String = params.sequence().next().unwrap();
            let helper = self.helper.lock().unwrap();

            for tx in helper.node.latest_block().txs {
                if tx.hash().to_0x() == hash {
                    match tx {
                        SupportedTransaction::MultiVm(_) => {
                            unreachable!("eth_getTransactionByHash for multiVM tx!")
                        }
                        SupportedTransaction::Evm(bytes) => {
                            let bytes = bytes.clone();
                            let rlp = Rlp::new(&bytes);
                            let (tx_request, sig) =
                                TransactionRequest::decode_signed_rlp(&rlp).unwrap();
                            let result = Some(EthTransaction::from(
                                tx_request,
                                sig,
                                hash,
                                helper.node.latest_block(),
                            ));
                            info!("==== {:#?}", result);
                            return result;
                        }
                    }
                }
            }

            return None;
        })?;
        module.register_method("eth_sendRawTransaction", |params, _| {
            let params_str = format!("{:#?}", params);
            info!(
                "eth_sendRawTransaction {:#?}",
                if params_str.len() > 100 {
                    "<params too long>"
                } else {
                    params_str.as_str()
                }
            );

            let data_str: String = params.sequence().next::<String>().unwrap().from_0x();
            let mut helper = self.helper.lock().unwrap();
            let node = &mut helper.node;
            let data = hex::decode(data_str).unwrap();

            let rlp = ethers_core::utils::rlp::Rlp::new(&data);
            let (tx, sig) = ethers_core::types::TransactionRequest::decode_signed_rlp(&rlp).unwrap();

            match sig.verify(tx.sighash(), tx.from.unwrap()) {
                Ok(_) => {},
                Err(error) => info!("Invalid signature {:#?}", error),
            }
            
            let tx = SupportedTransaction::Evm(data);
            let hash = tx.hash();
            node.add_tx(tx);
            node.produce_block(true);

            hash.to_0x()
        })?;

        module.register_method("mvm_createAccount", |params, _| {
            info!("mvm_createAccount: {:#?}", params);

            let obj: HashMap<String, String> = params.sequence().next().unwrap();
            let multivm_name = obj.get("multivm").unwrap();
            let evm_public_key: String = obj.get("evm").unwrap().from_0x();
            let vk = VerifyingKey::from_sec1_bytes(&hex::decode(evm_public_key).unwrap()).unwrap();
            let multivm = MultiVmAccountId::try_from(multivm_name.to_string()).unwrap();

            let mut helper = self.helper.lock().unwrap();
            let account = helper.create_evm_account(&multivm, vk);
            info!("=== Account added: {:#?}", account);
            account.to_0x()
        })?;

        module.register_method("eth_call", |params, _| {
            info!("eth_call: {:#?}", params.sequence());

            let helper = self.helper.lock().unwrap();

            let obj: HashMap<String, String> = params.sequence().next().expect("failed");

            let from: Option<H160> = obj.get("from").map(|from| from.from_0x());
            let to: H160 = obj.get("to").unwrap().from_0x();
            let data: String = obj.get("data").unwrap().from_0x();
            let payload = hex::decode(data).unwrap();
            let view = SupportedView::Evm(EvmCall {
                from: from.map(|f| f.0),
                to: to.0,
                input: payload,
            });
            let result = helper.node.view(view);
            let deserialized: Vec<u8> =
                BorshDeserialize::deserialize(&mut result.as_slice()).unwrap();
            info!("==== returned {:#?}", deserialized.to_0x());
            deserialized.to_0x()
        })?;

        for method in METHODS.iter() {
            module.register_method(method, move |_, _| {
                info!("{}", method);
            })?;
        }

        let address = server.local_addr()?;
        let handle = server.start(module);
        info!("Server started at http://{}", address);

        handle.stopped().await;

        Ok(())
    }
}

lazy_static! {
static ref METHODS: Vec<&'static str> = vec![
    "web3_sha3",
    // "net_version",
    "net_listening",
    "net_peerCount",
    "eth_protocolVersion",
    "eth_syncing",
    "eth_coinbase",
    // "eth_chainId",
    "eth_mining",
    "eth_hashrate",
    // "eth_gasPrice",
    "eth_accounts",
    // "eth_blockNumber",
    // "eth_getBalance",
    "eth_getStorageAt",
    // "eth_getTransactionCount",
    "eth_getBlockTransactionCountByHash",
    "eth_getBlockTransactionCountByNumber",
    "eth_getUncleCountByBlockHash",
    "eth_getUncleCountByBlockNumber",
    // "eth_getCode",
    "eth_sign",
    "eth_signTransaction",
    // "eth_sendTransaction",
    // "eth_sendRawTransaction",
    // "eth_call",
    // "eth_estimateGas",
    // "eth_getBlockByHash",
    // "eth_getBlockByNumber",
    // "eth_getTransactionByHash",
    "eth_getTransactionByBlockHashAndIndex",
    "eth_getTransactionByBlockNumberAndIndex",
    // "eth_getTransactionReceipt",
    "eth_getUncleByBlockHashAndIndex",
    "eth_getUncleByBlockNumberAndIndex",
    "eth_newFilter",
    "eth_newBlockFilter",
    "eth_newPendingTransactionFilter",
    "eth_uninstallFilter",
    "eth_getFilterChanges",
    "eth_getFilterLogs",
    "eth_getLogs",
];
}