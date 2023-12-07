benchmarks:
	cd example_contracts && cargo +nightly build --release
	cd multivm_core && RUST_LOG=info cargo +nightly run --release --bin benchmarks

build_example_contracts:
	cd example_contracts && cargo +nightly build

start_server:
	cd multivm_core && cargo +nightly run --bin server

deploy:
	cd multivm_core/client && npx hardhat run scripts/deploy.js --network multivm

fmt:
	cd multivm_core && cargo fmt
