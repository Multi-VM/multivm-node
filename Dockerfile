FROM rust:latest AS chef
RUN cargo install cargo-chef
WORKDIR app

FROM chef AS planner
COPY . .

WORKDIR multivm_core
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder

RUN apt-get update && \
    apt-get install -y \
        cmake \
        clang
        
RUN cargo install cargo-risczero
RUN cargo risczero install

COPY --from=planner /app/multivm_core/recipe.json recipe.json
COPY . .

FROM builder AS node-builder
WORKDIR /app/multivm_core
RUN cargo build --release --bin server

FROM debian:bookworm-slim AS mvm-node
WORKDIR node
COPY --from=node-builder /app/multivm_core/target/release/server /usr/local/bin
ENV RUST_LOG=info,risc0_zkvm=warn
EXPOSE 8080
ENTRYPOINT ["/usr/local/bin/server"]
