# MultiVM ZKP Node MVP

MultiVM ZKP Blockchain Node is our blockchain prototype, which utilizes zero-knowledge proofs (via [risc0](https://github.com/risc0/risc0)) for all state transitions.

Architecture description, playground cases, tech details and other documentation could be found on [MultiVM Docs](https://docs.multivm.io/)

## Structure

- `multivm_core` - the core of the node, currently contains prototype of the runtime.
- `multivm_sdk` - SDK for writing contracts.
- `example_contracts` - example contracts written using the SDK.

## Playgrounds

### Run

Run the playground.
```sh
cd multivm_core

cargo run --release --bin erc20
# or
cargo run --release --bin example_token
```
## Docker

```sh
docker build -t multivm .

docker run -d -p 8080:8080 multivm
```
