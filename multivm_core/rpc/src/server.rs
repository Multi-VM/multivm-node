use std::{collections::HashMap, str::FromStr, sync::RwLock};

use eth_primitive_types::H160;
use ethers_core::k256::ecdsa::SigningKey;
use hyper::Method;
use jsonrpsee::RpcModule;
use jsonrpsee::{
    server::Server,
    types::{error::ErrorCode, ErrorObject, ErrorObjectOwned},
};
use lazy_static::lazy_static;
use multivm_primitives::{
    AccountId, EthereumTransactionRequest, EvmAddress, MultiVmAccountId, SolanaAddress,
    SupportedTransaction,
};
use multivm_runtime::viewer::{EvmCall, SupportedView};
use playgrounds::NodeHelper;
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use tracing::{debug, info, span, Level};

use crate::utils::{EthBlockOutput, EthTransaction, EthTransactionReceipt, From0x, To0x};

static CHAIN_ID: u64 = 1044942;

#[derive(Clone)]
pub struct MultivmServer {}

impl MultivmServer {
    pub fn new() -> Self {
        Self {}
    }

    pub async fn start(&self, db_path: Option<String>, port: u16) -> anyhow::Result<()> {
        let cors = CorsLayer::new()
            .allow_methods([Method::POST, Method::OPTIONS])
            .allow_origin(Any)
            .allow_headers([hyper::header::CONTENT_TYPE]);
        let middleware = tower::ServiceBuilder::new().layer(cors);
        let server = Server::builder()
            .set_middleware(middleware)
            .build(format!("0.0.0.0:{}", port))
            .await?;
        let helper = NodeHelper::new(db_path);
        let mut module = RpcModule::new(RwLock::new(helper));

        register_method(&mut module, "eth_chainId", |_, _| Ok(CHAIN_ID.to_0x()))?;

        register_method(&mut module, "eth_blockNumber", move |_, ctx| {
            let height = format!(
                "0x{:x}",
                ctx.read()
                    .map_err(internal_error)?
                    .node
                    .latest_block()
                    .height
            );
            Ok(height)
        })?;
        register_method(&mut module, "eth_getBalance", move |params, ctx| {
            let address: H160 = params
                .sequence()
                .next::<String>()
                .map_err(invalid_params)?
                .from_0x();
            let account = ctx
                .read()
                .map_err(internal_error)?
                .account(&AccountId::Evm(address.into()));
            let balance = account.map(|a| a.balance).unwrap_or_default();
            Ok(balance.to_0x())
        })?;
        register_method(&mut module, "eth_getBlockByNumber", move |params, ctx| {
            let block_request = params.sequence().next::<String>().map_err(invalid_params)?;
            let helper = ctx.read().map_err(internal_error)?;
            let height = if block_request == "latest" {
                helper.node.latest_block().height
            } else {
                block_request.from_0x()
            };
            let output = helper
                .node
                .block_by_height(height)
                .map(|block| EthBlockOutput::from(&block));
            Ok(json!(output))
        })?;
        register_method(&mut module, "eth_getBlockByHash", move |params, ctx| {
            let block_request = params.sequence().next::<String>().map_err(invalid_params)?;
            let helper = ctx.read().map_err(internal_error)?;
            let _height = if block_request == "latest" {
                helper.node.latest_block().height
            } else {
                block_request.from_0x()
            };
            let block = helper.node.latest_block();
            let output = EthBlockOutput::from(&block);
            Ok(json!(output))
        })?;
        register_method(
            &mut module,
            "eth_getTransactionReceipt",
            move |params, ctx| {
                let hash = params.sequence().next::<String>().map_err(invalid_params)?;
                let helper = ctx.read().map_err(internal_error)?;
                for height in 1..=helper.node.latest_block().height {
                    let block = helper
                        .node
                        .block_by_height(height)
                        .ok_or(internal_error("block not found"))?;
                    for tx in block.txs.clone() {
                        if tx.hash().to_0x() == hash {
                            let receipt = EthTransactionReceipt::from(&tx, hash, &block);
                            return Ok(json!(receipt));
                        }
                    }
                }
                Ok(json!([]))
            },
        )?;

        register_method(&mut module, "eth_sendTransaction", move |_params, _| {
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
            Ok(())
        })?;

        register_method(&mut module, "net_version", |_, _| Ok("1"))?;
        register_method(&mut module, "eth_gasPrice", |_, _| Ok("0x1dfd14000"))?;
        register_method(&mut module, "eth_getCode", |_, _| Ok("0x1dfd14000"))?;
        register_method(&mut module, "eth_estimateGas", |_, _| Ok("0x5208"))?;
        register_method(
            &mut module,
            "eth_getTransactionCount",
            move |params, ctx| {
                let address = params.sequence().next::<String>().map_err(invalid_params)?;
                let account_id: AccountId = EvmAddress::try_from(address)
                    .map(|a| a.into())
                    .map_err(invalid_params)?;

                let nonce = ctx
                    .read()
                    .map_err(internal_error)?
                    .account(&account_id)
                    .map(|a| a.nonce)
                    .unwrap_or_default();

                Ok(nonce.to_0x())
            },
        )?;
        register_method(
            &mut module,
            "eth_getTransactionByHash",
            move |params, ctx| {
                let hash: String = params.sequence().next().map_err(invalid_params)?;
                let helper = ctx.read().map_err(internal_error)?;

                for tx in helper.node.latest_block().txs {
                    if tx.hash().to_0x() == hash {
                        match tx {
                            SupportedTransaction::MultiVm(_) => {
                                unreachable!("eth_getTransactionByHash for multiVM tx!")
                            }
                            SupportedTransaction::Evm(tx) => {
                                let (tx_request, sig) = tx.decode();
                                let result = Some(EthTransaction::from(
                                    tx_request,
                                    sig,
                                    hash,
                                    helper.node.latest_block(),
                                ));
                                return Ok(result);
                            }
                            SupportedTransaction::Solana(_) => {
                                unreachable!("eth_getTransactionByHash for Solana tx!")
                            }
                        }
                    }
                }
                return Ok(None);
            },
        )?;
        register_method(&mut module, "eth_sendRawTransaction", move |params, ctx| {
            let data_str: String = params
                .sequence()
                .next::<String>()
                .map_err(invalid_params)?
                .from_0x();

            let data = hex::decode(data_str).map_err(invalid_params)?;

            let rlp = ethers_core::utils::rlp::Rlp::new(&data);
            let (tx, sig) = ethers_core::types::TransactionRequest::decode_signed_rlp(&rlp)
                .map_err(invalid_params)?;

            match sig.verify(
                tx.sighash(),
                tx.from.ok_or(invalid_params("tx.from is required"))?,
            ) {
                Ok(_) => {}
                Err(error) => {
                    debug!(?error, "invalid signature");

                    Err(ErrorObject::owned(
                        ErrorCode::InvalidRequest.code(),
                        ErrorCode::InvalidRequest.message(),
                        Some("invalid signature"),
                    ))?;
                }
            }

            let mut helper = ctx.write().map_err(internal_error)?;

            let tx = SupportedTransaction::Evm(EthereumTransactionRequest::new(data));
            let hash = tx.hash();
            helper.node.add_tx(tx);
            helper.node.produce_block(true);

            Ok(hash.to_0x())
        })?;

        register_method(&mut module, "mvm_debugAirdrop", move |params, ctx| {
            let obj: HashMap<String, String> = params.sequence().next().map_err(invalid_params)?;
            let multivm_name = obj
                .get("multivm")
                .map(|s| s.clone())
                .ok_or(invalid_params("multivm field is required"))?;
            let address_str: String = obj
                .get("address")
                .map(|s| s.clone())
                .ok_or(invalid_params("address field is required"))?
                .from_0x();
            let address_bytes: [u8; 20] = hex::decode(address_str)
                .map_err(invalid_params)?
                .try_into()
                .map_err(|_| invalid_params("invalid address length"))?;
            let address: EvmAddress = address_bytes.into();
            let multivm =
                MultiVmAccountId::try_from(multivm_name.to_string()).map_err(invalid_params)?;

            let mut helper = ctx.write().map_err(internal_error)?;
            if let None = helper.account(&multivm.clone().into()) {
                helper.create_evm_account(&multivm, address.clone());
                debug!(
                    multivm_id = ?multivm,
                    evm_address = ?address,
                    "account created"
                );
            }
            helper.node.produce_block(true);
            Ok(address.to_string())
        })?;

        register_method(&mut module, "mvm_deployContract", move |params, ctx| {
            let obj: HashMap<String, String> = params.sequence().next().map_err(invalid_params)?;
            let bytecode: Vec<u8> = obj
                .get("bytecode")
                .ok_or(invalid_params("bytecode field is required"))?
                .from_0x();
            let multivm_name = obj
                .get("multivm")
                .map(|s| s.clone())
                .ok_or(invalid_params("multivm field is required"))?;
            let contract_type = obj
                .get("contract_type")
                .map(|s| s.clone())
                .ok_or(invalid_params("contract_type field is required"))?;
            let private_key: String = obj
                .get("private_key")
                .ok_or(invalid_params("private_key field is required"))?
                .from_0x();
            let sk = SigningKey::from_slice(&hex::decode(private_key).map_err(invalid_params)?)
                .map_err(invalid_params)?;
            let account_id =
                MultiVmAccountId::try_from(multivm_name.to_string()).map_err(invalid_params)?;

            let mut helper = ctx.write().map_err(internal_error)?;
            helper.deploy_contract_with_key(&account_id, contract_type, bytecode, sk);
            helper.produce_block(true);

            Ok("0x0")
        })?;

        register_method(&mut module, "mvm_viewCall", move |params, ctx| {
            let mut seq = params.sequence();
            let contract_name: String = seq.next().map_err(invalid_params)?;

            let contract_id = if let Ok(h160) = H160::from_str(contract_name.as_str()) {
                AccountId::Evm(EvmAddress::try_from(h160).map_err(invalid_params)?)
            } else {
                AccountId::MultiVm(
                    MultiVmAccountId::try_from(contract_name.clone()).map_err(invalid_params)?,
                )
            };

            let call = seq.next().map_err(invalid_params)?;
            let helper = ctx.read().map_err(internal_error)?;

            let result = helper.view(&contract_id.into(), call);
            match result {
                Ok(Ok(data)) => Ok(data.to_0x()),
                _ => Ok(0u128.to_0x()),
            }
        })?;

        register_method(&mut module, "eth_call", move |params, ctx| {
            let obj: HashMap<String, String> = params.sequence().next().map_err(invalid_params)?;

            let from: Option<H160> = obj.get("from").map(|from| from.from_0x());
            let to: H160 = obj
                .get("to")
                .ok_or(invalid_params("to field is required"))?
                .from_0x();
            let data: String = obj
                .get("data")
                .ok_or(invalid_params("data field is required"))?
                .from_0x();
            let payload = hex::decode(data).map_err(invalid_params)?;
            let view = SupportedView::Evm(EvmCall {
                from: from.map(|f| f.0),
                to: to.0,
                input: payload,
            });
            let helper = ctx.read().map_err(internal_error)?;
            let result = helper.node.contract_view(view);
            match result {
                Ok(Ok(data)) => {
                    let response: Vec<u8> = borsh::from_slice(&data).map_err(internal_error)?;
                    Ok(response.to_0x())
                }
                _ => Ok(0u128.to_0x()),
            }
        })?;

        register_method(&mut module, "mvm_accountInfo", move |params, ctx| {
            let mut seq = params.sequence();
            let address_name: String = seq.next().map_err(invalid_params)?;

            let account_id = if let Ok(h160) = H160::from_str(address_name.as_str()) {
                AccountId::Evm(EvmAddress::try_from(h160).map_err(invalid_params)?)
            } else {
                AccountId::MultiVm(
                    MultiVmAccountId::try_from(address_name.clone()).map_err(invalid_params)?,
                )
            };

            let account = ctx.read().map_err(internal_error)?.account(&account_id);
            Ok(json!(account))
        })?;

        register_method(&mut module, "svm_accountData", move |params, ctx| {
            let mut seq = params.sequence();
            let contract_address: String = seq.next().map_err(invalid_params)?;
            let storage_address: String = seq.next().map_err(invalid_params)?;

            let contract_address: SolanaAddress =
                contract_address.parse().map_err(invalid_params)?;
            let storage_address: SolanaAddress = storage_address.parse().map_err(invalid_params)?;

            let data = ctx
                .read()
                .map_err(internal_error)?
                .node
                .account_raw_storage(contract_address.into(), storage_address.to_string());

            Ok(json!(data))
        })?;

        for method in METHODS.iter() {
            register_method(&mut module, method, move |_, _| Ok(()))?;
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

fn register_method<F, R>(
    module: &mut RpcModule<RwLock<NodeHelper>>,
    method_name: &'static str,
    callback: F,
) -> Result<(), anyhow::Error>
where
    R: jsonrpsee::IntoResponse + serde::Serialize + std::fmt::Debug + Clone + 'static,
    F: Fn(jsonrpsee::types::Params, &RwLock<NodeHelper>) -> Result<R, ErrorObjectOwned>
        + Send
        + Sync
        + 'static,
{
    module.register_method(method_name, move |p, c| {
        let span = span!(
            Level::DEBUG,
            "rpc",
            method = method_name,
            id = generate_request_id()
        );
        let _enter = span.enter();

        {
            let params_debug = format!("{:?}", p);
            let params_debug = if params_debug.len() > 200 {
                "<params too long to log>"
            } else {
                params_debug.as_str()
            };
            debug!(params = params_debug);
        }

        let resp = callback(p, c);
        {
            let resp_debug = format!("{:?}", resp);
            let resp_debug = if resp_debug.len() > 200 {
                "<response too long to log>"
            } else {
                resp_debug.as_str()
            };
            debug!(response = resp_debug);
        }
        resp
    })?;

    Ok(())
}

fn generate_request_id() -> String {
    use rand::Rng;

    let request_id: String = rand::thread_rng()
        .sample_iter(&rand::distributions::Alphanumeric)
        .take(8)
        .map(char::from)
        .collect();

    request_id
}

fn internal_error(e: impl ToString) -> ErrorObjectOwned {
    ErrorObject::owned(
        ErrorCode::InternalError.code(),
        ErrorCode::InternalError.message(),
        Some(e.to_string()),
    )
}

fn invalid_params(e: impl ToString) -> ErrorObjectOwned {
    ErrorObject::owned(
        ErrorCode::InvalidParams.code(),
        ErrorCode::InvalidParams.message(),
        Some(e.to_string()),
    )
}
