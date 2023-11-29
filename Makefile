benchmarks:
	cd example_contracts && cargo build --release
	cd multivm_core && RUST_LOG=info cargo run --release --bin benchmarks

build_server:
	cd multivm_core && cargo +nightly build --release --bin server

build_example_contracts:
	cd example_contracts && cargo build --release

start_server:
	RUST_LOG=info multivm_core/target/release/server

deploy:
	cd multivm_core/client && npx hardhat run scripts/deploy.js --network multivm

fmt:
	cd multivm_core && cargo fmt
