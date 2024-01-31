use async_graphql::{
    http::GraphiQLSource, ComplexObject, Context, EmptyMutation, EmptySubscription, Enum, Object,
    Scalar, ScalarType, Schema, SimpleObject,
};
use async_graphql_poem::*;
use color_eyre::{eyre::eyre, Result};
use poem::{
    get, handler, listener::TcpListener, middleware::Cors, web::Html, EndpointExt, IntoResponse,
    Route, Server,
};
use tokio::task::JoinHandle;

use std::str::FromStr;

use multivm_primitives::{AccountId, MultiVmAccountId};

use super::{
    config::GraphqlConfig,
    storage::{models, Storage},
};

struct QueryRoot;

#[Object]
impl QueryRoot {
    /// Get latest block
    async fn latest_block<'ctx>(&self, ctx: &Context<'ctx>) -> Result<Block> {
        let storage = ctx.data::<Storage>().map_err(|err| eyre!(err.message))?;

        Ok(storage
            .find_latest_block()
            .await?
            .ok_or_else(|| eyre!("latest block not found"))?
            .try_into()?)
    }

    /// Get latest blocks
    async fn latest_blocks<'ctx>(&self, ctx: &Context<'ctx>, last: u64) -> Result<Vec<Block>> {
        let storage = ctx.data::<Storage>().map_err(|err| eyre!(err.message))?;

        let last = last.min(32);

        storage
            .latest_blocks(last.try_into()?)
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    /// Get block by hash or height
    async fn block<'ctx>(&self, ctx: &Context<'ctx>, slug: String) -> Result<Option<Block>> {
        let block_height: Option<i64> = slug.parse().ok();

        let storage = ctx.data::<Storage>().map_err(|err| eyre!(err.message))?;

        let block = match block_height {
            Some(block_height) => storage.find_block_by_number(block_height).await,
            None => storage.find_block_by_hash(&slug).await,
        }?
        .map(TryInto::try_into)
        .transpose()?;

        Ok(block)
    }

    /// Get latest stats
    async fn latest_stats<'ctx>(&self, ctx: &Context<'ctx>) -> Result<Stats> {
        let storage = ctx.data::<Storage>().map_err(|err| eyre!(err.message))?;

        Ok(storage
            .find_latest_stats()
            .await?
            .ok_or_else(|| eyre!("latest stats not found"))?
            .try_into()?)
    }

    /// Find transaction by hash
    async fn transaction<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        hash: String,
    ) -> Result<Option<Transaction>> {
        let hash = hash.replace("0x", "").to_lowercase();

        let storage = ctx.data::<Storage>().map_err(|err| eyre!(err.message))?;

        storage
            .find_transaction_by_hash(hash)
            .await?
            .map(TryInto::try_into)
            .transpose()
    }

    /// Find account by address
    async fn account<'ctx>(
        &self,
        ctx: &Context<'ctx>,
        vm: Vm,
        address: String,
    ) -> Result<Option<Account>> {
        let storage = ctx.data::<Storage>().map_err(|err| eyre!(err.message))?;

        Ok(match vm {
            Vm::EVM => {
                storage
                    .find_account_by_evm_address(address.replace("0x", ""))
                    .await
            }
            Vm::FVM => storage.find_account_by_fvm_address(address).await,
            Vm::SVM => storage.find_account_by_svm_address(address).await,
        }?
        .map(TryInto::try_into)
        .transpose()?)
    }
}

#[derive(SimpleObject)]
#[graphql(complex)]
struct Block {
    #[graphql(skip)]
    id: i64,
    height: u64,
    hash: String,
    timestamp: DateTime,
}

#[ComplexObject]
impl Block {
    async fn transactions<'ctx>(&self, ctx: &Context<'ctx>) -> Result<Vec<Transaction>> {
        let storage = ctx.data::<Storage>().map_err(|err| eyre!(err.message))?;

        storage
            .find_transactions_by_block_id(self.id)
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }
}

impl TryFrom<models::Block> for Block {
    type Error = color_eyre::eyre::Error;

    fn try_from(block: models::Block) -> Result<Self> {
        Ok(Self {
            id: block.id,
            height: block.number.try_into()?,
            hash: format!("0x{}", block.hash),
            timestamp: DateTime(block.timestamp),
        })
    }
}

#[derive(SimpleObject)]
#[graphql(complex)]
struct Transaction {
    #[graphql(skip)]
    id: i64,

    #[graphql(skip)]
    block_id: i64,

    #[graphql(skip)]
    signer_account_id: i64,

    #[graphql(skip)]
    receiver_account_id: i64,

    hash: String,
    format: Vm,
    nonce: u64,
}

#[ComplexObject]
impl Transaction {
    async fn block<'ctx>(&self, ctx: &Context<'ctx>) -> Result<Block> {
        let storage = ctx.data::<Storage>().map_err(|err| eyre!(err.message))?;

        Ok(storage
            .find_block_by_id(self.block_id)
            .await?
            .ok_or_else(|| eyre!("block not found"))?
            .try_into()?)
    }

    async fn signer<'ctx>(&self, ctx: &Context<'ctx>) -> Result<Account> {
        let storage = ctx.data::<Storage>().map_err(|err| eyre!(err.message))?;

        Ok(storage
            .find_account_by_id(self.signer_account_id)
            .await?
            .ok_or_else(|| eyre!("signer account not found"))?
            .try_into()?)
    }

    async fn receiver<'ctx>(&self, ctx: &Context<'ctx>) -> Result<Account> {
        let storage = ctx.data::<Storage>().map_err(|err| eyre!(err.message))?;

        Ok(storage
            .find_account_by_id(self.receiver_account_id)
            .await?
            .ok_or_else(|| eyre!("receiver account not found"))?
            .try_into()?)
    }

    async fn receipt<'ctx>(&self, ctx: &Context<'ctx>) -> Result<Receipt> {
        let storage = ctx.data::<Storage>().map_err(|err| eyre!(err.message))?;

        Ok(storage
            .find_root_receipt(self.id)
            .await?
            .ok_or_else(|| eyre!("receipt not found"))?
            .try_into()?)
    }
}

#[derive(SimpleObject)]
#[graphql(complex)]
struct Receipt {
    #[graphql(skip)]
    pub id: i64,

    #[graphql(skip)]
    pub contract_account_id: i64,

    // pub idx: u64,
    pub result: bool,
    pub response: Option<String>,
    pub gas_used: u64,
    pub call_method: String,
    pub call_args: String,
    pub call_gas: u64,
    pub call_deposit: String,
}

#[ComplexObject]
impl Receipt {
    async fn contract<'ctx>(&self, ctx: &Context<'ctx>) -> Result<Account> {
        let storage = ctx.data::<Storage>().map_err(|err| eyre!(err.message))?;

        Ok(storage
            .find_account_by_id(self.contract_account_id)
            .await?
            .ok_or_else(|| eyre!("contract account not found"))?
            .try_into()?)
    }

    async fn cross_calls_receipts<'ctx>(&self, ctx: &Context<'ctx>) -> Result<Vec<Receipt>> {
        let storage = ctx.data::<Storage>().map_err(|err| eyre!(err.message))?;

        storage
            .find_cross_calls_receipt(self.id)
            .await?
            .into_iter()
            .map(TryInto::try_into)
            .collect()
    }

    async fn events<'ctx>(&self, ctx: &Context<'ctx>) -> Result<Vec<Event>> {
        let storage = ctx.data::<Storage>().map_err(|err| eyre!(err.message))?;

        Ok(storage
            .find_events_by_receipt_id(self.id)
            .await?
            .into_iter()
            .map(|event| Event {
                _id: event.id,
                _receipt_id: event.receipt_id,
                // idx: event.index_in_receipt.try_into().unwrap(),
                message: event.message,
            })
            .collect())
    }
}

impl TryFrom<models::Receipt> for Receipt {
    type Error = color_eyre::eyre::Error;

    fn try_from(receipt: models::Receipt) -> Result<Self> {
        Ok(Self {
            id: receipt.id,
            // idx: receipt.index_in_transaction.try_into()?,
            result: receipt.result,
            response: receipt.response,
            gas_used: receipt.gas_used.try_into()?,
            contract_account_id: receipt.contract_account_id,
            call_method: receipt.call_method,
            call_args: receipt.call_args,
            call_gas: receipt.call_gas.try_into()?,
            call_deposit: receipt.call_deposit,
        })
    }
}

impl TryFrom<models::Transaction> for Transaction {
    type Error = color_eyre::eyre::Error;

    fn try_from(tx: models::Transaction) -> Result<Self> {
        Ok(Self {
            id: tx.id,
            hash: format!("0x{}", tx.hash),
            block_id: tx.block_id,
            signer_account_id: tx.signer_account_id,
            receiver_account_id: tx.receiver_account_id,
            format: tx.format.parse::<Vm>()?,
            nonce: tx.nonce.try_into()?,
        })
    }
}

#[derive(SimpleObject)]
struct Event {
    #[graphql(skip)]
    pub _id: i64,
    #[graphql(skip)]
    pub _receipt_id: i64,
    // pub idx: i64,
    pub message: String,
}

#[derive(Enum, Copy, Clone, Eq, PartialEq)]
enum Vm {
    EVM,
    FVM,
    SVM,
}

impl FromStr for Vm {
    type Err = color_eyre::eyre::Error;

    fn from_str(s: &str) -> Result<Self> {
        match s.to_uppercase().as_str() {
            "EVM" => Ok(Self::EVM),
            "FVM" => Ok(Self::FVM),
            "SVM" => Ok(Self::SVM),
            _ => Err(eyre!("invalid vm type")),
        }
    }
}

#[derive(SimpleObject)]
#[graphql(complex)]
struct Account {
    #[graphql(skip)]
    pub _id: i64,

    #[graphql(skip)]
    pub created_at_block_id: i64,

    #[graphql(skip)]
    pub modified_at_block_id: i64,

    pub fvm_address: Option<String>,
    pub evm_address: String,
    pub svm_address: String,
    pub executable_type: Option<Vm>,
    pub native_balance: String,
    pub meta_contract: bool,
}

#[ComplexObject]
impl Account {
    async fn created_at_block<'ctx>(&self, ctx: &Context<'ctx>) -> Result<Block> {
        let storage = ctx.data::<Storage>().map_err(|err| eyre!(err.message))?;

        Ok(storage
            .find_block_by_id(self.created_at_block_id)
            .await?
            .ok_or_else(|| eyre!("block not found"))?
            .try_into()?)
    }

    async fn modified_at_block<'ctx>(&self, ctx: &Context<'ctx>) -> Result<Block> {
        let storage = ctx.data::<Storage>().map_err(|err| eyre!(err.message))?;

        Ok(storage
            .find_block_by_id(self.modified_at_block_id)
            .await?
            .ok_or_else(|| eyre!("block not found"))?
            .try_into()?)
    }
}

impl TryFrom<models::Account> for Account {
    type Error = color_eyre::eyre::Error;
    fn try_from(account: models::Account) -> Result<Self> {
        let is_meta_contract = account
            .fvm_address
            .clone()
            .map(|fvm| {
                MultiVmAccountId::try_from(fvm)
                    .map(|fvm| AccountId::from(fvm) == AccountId::system_meta_contract())
                    .unwrap_or_default()
            })
            .unwrap_or_default();

        Ok(Account {
            _id: account.id,
            created_at_block_id: account.created_at_block_id,
            modified_at_block_id: account.modified_at_block_id,
            fvm_address: account.fvm_address,
            evm_address: format!("0x{}", account.evm_address),
            svm_address: account.svm_address,
            executable_type: account.executable_type.map(|e| e.parse().unwrap()),
            native_balance: account.native_balance,
            meta_contract: is_meta_contract,
        })
    }
}

#[derive(SimpleObject)]
#[graphql(complex)]
struct Stats {
    pub timestamp: DateTime,
    #[graphql(skip)]
    pub block_id: i64,
    pub total_txs: u64,
    pub total_accounts: i64,
    pub total_contracts: u64,
}

#[ComplexObject]
impl Stats {
    async fn at_block<'ctx>(&self, ctx: &Context<'ctx>) -> Result<Block> {
        let storage = ctx.data::<Storage>().unwrap();
        Ok(storage
            .find_block_by_id(self.block_id)
            .await?
            .ok_or_else(|| eyre!("latest block not found"))?
            .try_into()?)
    }
}

impl TryFrom<models::Stats> for Stats {
    type Error = color_eyre::eyre::Error;

    fn try_from(stats: models::Stats) -> Result<Self> {
        Ok(Stats {
            timestamp: DateTime(stats.timestamp),
            block_id: stats.block_id.try_into()?,
            total_txs: stats.total_txs.try_into()?,
            total_accounts: stats.total_accounts.try_into()?,
            total_contracts: stats.total_contracts.try_into()?,
        })
    }
}

struct DateTime(i64);

#[Scalar]
impl ScalarType for DateTime {
    fn parse(value: async_graphql::Value) -> async_graphql::InputValueResult<Self> {
        if let async_graphql::Value::String(value) = &value {
            chrono::DateTime::parse_from_rfc3339(&value)
                .map(|dt| DateTime(dt.timestamp_millis()))
                .map_err(|_| async_graphql::InputValueError::custom("invalid date time format"))
        } else {
            Err(async_graphql::InputValueError::expected_type(value))
        }
    }

    fn to_value(&self) -> async_graphql::Value {
        let ts_secs = self.0 / 1000;
        let ts_ns = (self.0 % 1000) * 1_000_000;
        let dt = chrono::DateTime::from_timestamp(ts_secs, ts_ns.try_into().unwrap()).unwrap();
        async_graphql::Value::String(dt.to_rfc3339())
    }
}

#[handler]
async fn graphiql() -> impl IntoResponse {
    Html(GraphiQLSource::build().finish())
}

pub async fn start(config: GraphqlConfig, storage: Storage) -> Result<JoinHandle<Result<()>>> {
    let schema = Schema::build(QueryRoot, EmptyMutation, EmptySubscription)
        .limit_depth(config.depth_limit)
        .limit_complexity(config.complexity_limit)
        .data(storage)
        .finish();

    let app = Route::new()
        .at("/", get(graphiql).post(GraphQL::new(schema)))
        .with(Cors::new());

    let handle = tokio::spawn(async move {
        Server::new(TcpListener::bind(config.bind_address))
            .run(app)
            .await
            .map_err(|e| eyre!(e))
    });

    Ok(handle)
}
