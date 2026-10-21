# syntax=docker/dockerfile:1
# A static musl binary and the website on a distroless base: no shell, no package manager.

FROM rust:1-alpine AS build
RUN apk add --no-cache gcc musl-dev
# Embeds the dependency list in the binary, so Trivy can scan the Rust crates inside it.
RUN cargo install cargo-auditable --version 0.7.7 --locked
# Must equal wasm-bindgen in Cargo.lock (web/test/versions.test.ts checks this line).
RUN cargo install wasm-bindgen-cli --version 0.2.129 --locked
WORKDIR /src
COPY . .
# rust-toolchain.toml adds the wasm32-unknown-unknown target.
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo auditable build --release --locked --bin whichride-server \
    && cp target/release/whichride-server /whichride-server \
    && cargo build --profile web --locked -p whichride-wasm --target wasm32-unknown-unknown \
    && wasm-bindgen --target web --out-dir /wasm target/wasm32-unknown-unknown/web/whichride_wasm.wasm

FROM node:24-alpine AS web
WORKDIR /src/web
COPY web/package.json web/package-lock.json ./
RUN npm ci
COPY web/ ./
# The site reads the app's wording and the seed prices at build time.
COPY android/app/src/main/res/ /src/android/app/src/main/res/
COPY crates/server/catalogue.json src/generated/catalogue.json
COPY --from=build /wasm src/generated/wasm
RUN npm run build

FROM gcr.io/distroless/static-debian13:nonroot
COPY --from=build /whichride-server /usr/local/bin/whichride-server
COPY --from=web /src/web/dist /srv/web
ENV BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=info \
    WEB_DIR=/srv/web
EXPOSE 8080
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/whichride-server"]
CMD ["serve"]
