pub use solana_program::account_info;
pub use solana_program::instruction;
pub use solana_program::program;
pub use solana_program::program_error;
pub use solana_program::program_pack;
pub use solana_program::pubkey;
pub use solana_program::rent;
pub use solana_program::system_instruction;
pub use solana_program::system_program;
pub use solana_program::sysvar;

pub use borsh;
pub use multivm_primitives;
pub use multivm_sdk;
pub use risc0_zkvm;

use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub struct SolanaContext {
    pub accounts: Vec<[u8; 32]>,
    pub instruction_data: Vec<u8>,
}

impl SolanaContext {
    pub fn pubkeys(&self) -> Vec<solana_program::pubkey::Pubkey> {
        self.accounts
            .iter()
            .map(|x| solana_program::pubkey::Pubkey::new_from_array(x.clone()))
            .collect()
    }
}

pub mod entrypoint {

    pub use solana_program::entrypoint::ProgramResult;

    #[macro_export]
    macro_rules! entrypoint {
        ($path:path) => {
            // Type check the given path
            const ZKVM_ENTRY: fn(
                &solana_program::pubkey::Pubkey,
                &[solana_program::account_info::AccountInfo],
                &[u8],
            ) -> solana_program::entrypoint::ProgramResult = $path;

            // Include generated main in a module so we don't conflict
            // with any other definitions of "main" in this file.
            mod zkvm_generated_main {
                #[no_mangle]
                fn main() {
                    let context =
                        solana_program::multivm_primitives::ContractCallContext::try_from_bytes(
                            solana_program::risc0_zkvm::guest::env::read(),
                        )
                        .expect("Corrupted ContractCallContext");

                    solana_program::multivm_sdk::env::setup_env(&context);

                    let solana_context: solana_program::SolanaContext =
                        solana_program::borsh::from_slice(&context.contract_call.args)
                            .expect("Corrupted SolanaContext");

                    let program_id: solana_program::pubkey::Pubkey =
                        solana_program::pubkey::Pubkey::new_from_array(
                            context.contract_id.solana().to_bytes(),
                        );

                    let mut data: Vec<_> = solana_context
                        .pubkeys()
                        .into_iter()
                        .map(|key| {
                            let storage: Vec<u8> =
                                solana_program::multivm_sdk::env::get_storage(key.to_string())
                                    .unwrap_or_else(|| vec![0u8; 1024]);
                            (key, storage, 0u64)
                        })
                        .collect();

                    let accounts: Vec<solana_program::account_info::AccountInfo> = data
                        .iter_mut()
                        .map(
                            |(pubkey, data, lamports)| solana_program::account_info::AccountInfo {
                                data: std::rc::Rc::new(std::cell::RefCell::new(
                                    data.as_mut_slice(),
                                )),
                                key: pubkey,
                                owner: &program_id,
                                rent_epoch: 0,
                                is_signer: false,
                                is_writable: true,
                                executable: false,
                                lamports: std::rc::Rc::new(std::cell::RefCell::new(lamports)),
                            },
                        )
                        .collect();

                    let instruction_data: &[u8] = &solana_context.instruction_data;

                    match super::ZKVM_ENTRY(&program_id, &accounts, instruction_data) {
                        Ok(_) => {}
                        Err(e) => {
                            panic!("{}", e)
                        }
                    }

                    for (key, data, lamports) in data {
                        println!("updating storage {:?}", key);
                        solana_program::multivm_sdk::env::set_storage(
                            key.to_string(),
                            data.as_slice(),
                        );
                    }

                    solana_program::multivm_sdk::env::commit(());
                }
            }
        };
    }
}

#[macro_export]
macro_rules! msg {
    ($msg:expr) => {
        println!("{}", $msg)
    };
    ($($arg:tt)*) => (println!("{}", &format!($($arg)*)));
}
