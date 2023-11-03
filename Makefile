r:
	cd example_contracts && cargo build --release
	cd multivm_core && RUST_LOG=info cargo run --release --bin benchmarks

rf:
	cd example_contracts && cargo build --release
	cd multivm_core && RUST_LOG=info cargo run --release --bin fibonacci

fmt:
	cd multivm_core && cargo fmt
