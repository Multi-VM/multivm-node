benchmarks:
	cd example_contracts && cargo build --release
	cd multivm_core && RUST_LOG=info cargo run --release --bin benchmarks

start_server:
	cd multivm_core && cargo +nightly-2023-03-06 build --release --bin server
	sudo RUST_LOG=info multivm_core/target/release/server

deploy:
	cd multivm_core/client && npx hardhat run scripts/deploy.js --network multivm

fmt:
	cd multivm_core && cargo fmt
