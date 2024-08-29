FROM rust:alpine AS builder

WORKDIR /usr/src/beerbot

RUN apk add --no-cache musl-dev pkgconfig

#Cache deps as layer
COPY Cargo.lock Cargo.toml ./
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    mkdir src \
    && echo "// dummy file" > src/lib.rs \
    && cargo build --target=x86_64-unknown-linux-musl --release

COPY src src

RUN --mount=type=cache,target=/usr/local/cargo/registry cargo build --target=x86_64-unknown-linux-musl --release

FROM alpine
COPY --from=builder /usr/src/beerbot/target/x86_64-unknown-linux-musl/release/beer-bot /usr/local/bin/beer-bot

CMD ["beer-bot", "/run/secrets/config.toml"]