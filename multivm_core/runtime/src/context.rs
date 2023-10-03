// use anyhow::Result;
// use risc0_zkvm::Session;
// use tracing::{debug, field::debug};

// use std::{
//     str::FromStr,
//     sync::{Arc, RwLock},
// };

// use multivm_primitives::{syscalls::CrossContractCallRequest, AccountId, ContractEntrypointContext};

// pub struct ExecutionContext {
//     call: ContractEntrypointContext,
//     used_gas: u64,

//     pub db: sled::Db,

//     cross_contract_calls: Vec<Arc<RwLock<ExecutionContext>>>,
//     session: Option<Session>,
// }

// impl ExecutionContext {
//     pub fn new(call: ContractEntrypointContext, db: sled::Db) -> Self {
//         Self {
//             call,
//             used_gas: 0,
//             db,
//             cross_contract_calls: Vec::new(),
//             session: None,
//         }
//     }

//     pub fn cross_contract_call(
//         &mut self,
//         req: CrossContractCallRequest,
//     ) -> Result<Arc<RwLock<ExecutionContext>>> {
//         let CrossContractCallRequest {
//             contract: account,
//             method,
//             args,
//             attached_gas,
//         } = req;

//         // if self.available_gas() < req.attached_gas {
//         //     return Err(anyhow::anyhow!(
//         //         "Not enough gas, available: {}, attached: {}",
//         //         self.available_gas(),
//         //         req.attached_gas
//         //     ));
//         // }

//         let call = ContractEntrypointContext {
//             account,
//             method,
//             args,
//             attached_gas,
//             sender: self.call.account.clone(),
//             signer: self.call.signer.clone(),
//         };

//         let context = Arc::new(RwLock::new(ExecutionContext {
//             call,
//             used_gas: 0,
//             db: self.db.clone(),
//             cross_contract_calls: Vec::new(),
//             session: None,
//         }));

//         self.cross_contract_calls.push(context.clone());

//         Ok(context)
//     }

//     pub fn call(&self) -> &ContractEntrypointContext {
//         &self.call
//     }

//     pub fn used_gas(&self) -> u64 {
//         self.used_gas
//     }

//     pub fn available_gas(&self) -> u64 {
//         let cc_gas = self
//             .cross_contract_calls
//             .iter()
//             .map(|call| call.read().unwrap().used_gas())
//             .sum::<u64>();

//         self.call
//             .attached_gas
//             .saturating_sub(self.used_gas)
//             .saturating_sub(cc_gas)
//     }

//     pub fn execution_session(&self) -> Option<&Session> {
//         self.session.as_ref()
//     }

//     pub fn set_execution_session(&mut self, session: Session) {
//         self.session = Some(session);
//     }

//     pub fn set_gas_usage(&mut self, used_gas: u64) {
//         self.used_gas = used_gas;
//     }

//     // TODO: remove hardcode, use custom alias system
//     pub fn get_account_evm_address(account_id: AccountId) -> eth_primitive_types::H160 {
//         let mut hardcoded_mappings = std::collections::HashMap::new();
//         hardcoded_mappings.insert(
//             AccountId::from(AccountId::new("alice.multivm".to_string())),
//             eth_primitive_types::H160::from_str("0x0FF1CE0000000000000000000000000000000001")
//                 .unwrap(),
//         );
//         hardcoded_mappings.insert(
//             AccountId::from(AccountId::new("bob.multivm".to_string())),
//             eth_primitive_types::H160::from_str("0x0FF1CE0000000000000000000000000000000002")
//                 .unwrap(),
//         );
//         hardcoded_mappings.insert(
//             AccountId::from(AccountId::new("charlie.multivm".to_string())),
//             eth_primitive_types::H160::from_str("0x0FF1CE0000000000000000000000000000000003")
//                 .unwrap(),
//         );
//         hardcoded_mappings.insert(
//             AccountId::from(AccountId::new("eve.multivm".to_string())),
//             eth_primitive_types::H160::from_str("0x0FF1CE0000000000000000000000000000000004")
//                 .unwrap(),
//         );

//         hardcoded_mappings.get(&account_id).unwrap().clone()
//     }
// }
