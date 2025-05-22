FROM lukemathwalker/cargo-chef:latest-rust-1.86-bookworm AS chef
WORKDIR /build

FROM chef AS planner
COPY . .
RUN cargo chef prepare  --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /build/recipe.json recipe.json
# Build dependencies - this is the caching Docker layer!
RUN cargo chef cook --release --recipe-path recipe.json
# Build application
COPY . .
ARG APP_NAME
RUN cargo build --release --bin "$APP_NAME"

FROM gcr.io/distroless/cc-debian12@sha256:c53c9416a1acdbfd6e09abba720442444a3d1a6338b8db850e5e198b59af5570
WORKDIR /app
ARG APP_NAME
ARG APP_CONFIG
ENV APP_CONFIG="$APP_CONFIG"
COPY --from=builder "/build/target/release/$APP_NAME" "bootstrap"
ENTRYPOINT ["/app/bootstrap"]
