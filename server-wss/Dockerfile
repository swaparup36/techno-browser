FROM rust:1.98-bookworm AS build

RUN apt-get update && apt-get install -y --no-install-recommends \
    clang libclang-dev libssl-dev pkg-config cmake && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /src
RUN git init && \
    git remote add origin https://github.com/rustdesk/rustdesk-server.git && \
    git fetch --depth=1 origin a7736be5e40f85bfc141120dce587e836e5d4b80 && \
    git checkout FETCH_HEAD && \
    git submodule update --init --recursive
COPY server.patch /tmp/server.patch
RUN git apply /tmp/server.patch && \
    cargo build --release --locked --bin hbbs --bin hbbr

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 libsqlite3-0 libsodium23 && \
    rm -rf /var/lib/apt/lists/*
COPY --from=build /src/target/release/hbbs /usr/local/bin/hbbs
COPY --from=build /src/target/release/hbbr /usr/local/bin/hbbr
WORKDIR /root
