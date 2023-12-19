pub mod env;
pub use multivm_primitives;
pub use multivm_sdk_macros::generate_payload;

#[macro_export]
macro_rules! entrypoint {
    ($path:path) => {
        // Type check the given path
        const ZKVM_ENTRY: fn(multivm_sdk::multivm_primitives::ContractCall) = $path;

        // Include generated main in a module so we don't conflict
        // with any other definitions of "main" in this file.
        mod zkvm_generated_main {
            use std::io::Read;

            #[no_mangle]
            fn main() {
                let mut bytes = Vec::<u8>::new();
                risc0_zkvm::guest::env::stdin()
                    .read_to_end(&mut bytes)
                    .unwrap();

                let context =
                    multivm_sdk::multivm_primitives::ContractCallContext::try_from_bytes(bytes)
                        .expect("Corrupted ContractCallContext");
                multivm_sdk::env::setup_env(&context);
                super::ZKVM_ENTRY(context.contract_call)
            }
        }
    };
}
