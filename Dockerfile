# The release image Render builds from the `release` branch (ADR 0006, ADR 0013).
# The SPA and `server` in the Cargo profile `release`, without `test-controls`, on distroless as `nonroot`.

FROM lukemathwalker/cargo-chef:0.1.78-rust-1.98.1-slim-trixie AS chef
# The base image holds the toolchain that rust-toolchain.toml pins; this keeps rustup from fetching rustfmt and Clippy.
ENV RUSTUP_TOOLCHAIN=1.98.1 SQLX_OFFLINE=true
WORKDIR /build

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS server
COPY --from=planner /build/recipe.json recipe.json
# The dependencies' layer, rebuilt only when Cargo.toml or Cargo.lock change.
RUN cargo chef cook --release --locked -p server --recipe-path recipe.json
COPY . .
RUN cargo build --release --locked -p server

FROM node:24.21.0-trixie-slim AS spa
RUN npm install --global pnpm@12.16.1
WORKDIR /build
COPY package.json pnpm-lock.yaml pnpm-workspace.yaml ./
COPY spa/package.json spa/
COPY contract/package.json contract/
COPY acceptance/package.json acceptance/
RUN pnpm install --frozen-lockfile --filter spa...
COPY assets assets
COPY contract contract
COPY spa spa
RUN pnpm --filter spa build

FROM gcr.io/distroless/cc-debian13:nonroot
WORKDIR /app
COPY --from=server /build/target/release/server /app/server
COPY --from=spa /build/spa/dist /app/spa/dist
ENV APP_DIR=/app/spa/dist
EXPOSE 10000
USER nonroot
ENTRYPOINT ["/app/server"]
