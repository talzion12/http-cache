FROM docker.io/library/rust:1.61.0-alpine AS chef
RUN apk add --no-cache musl-dev
RUN cargo install cargo-chef
WORKDIR app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
RUN apk add protoc
COPY --from=planner /app/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json
COPY . .
RUN cargo build --release --bin http-cache

FROM docker.io/library/alpine:3.15 AS runtime
WORKDIR app
COPY --from=builder /app/target/release/http-cache /usr/local/bin/http-cache
ENTRYPOINT ["http-cache"]