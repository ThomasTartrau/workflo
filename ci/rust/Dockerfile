FROM rust:1.88.0

ENV CARGO_BINSTALL_VERSION=1.12.5
ENV RUSTC_WRAPPER=sccache
ENV SCCACHE_DIR=/tmp/sccache

RUN apt-get update && apt-get install -y --no-install-recommends \
        lld \
    && rm -rf /var/lib/apt/lists/*

RUN curl -L --proto '=https' --tlsv1.2 -sSf \
        "https://github.com/cargo-bins/cargo-binstall/releases/download/v${CARGO_BINSTALL_VERSION}/cargo-binstall-x86_64-unknown-linux-musl.tgz" \
    | tar xz -C /usr/local/bin/

RUN cargo binstall --no-confirm \
        sccache \
        cargo-nextest \
        cargo-llvm-cov \
        cargo-audit

RUN rustup component add rustfmt clippy
