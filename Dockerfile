# syntax=docker/dockerfile:1
# A static musl binary on a distroless base: no shell, no package manager.

FROM rust:1-alpine AS build
RUN apk add --no-cache gcc musl-dev
# Embeds the dependency list in the binary, so Trivy can scan the Rust crates inside it.
RUN cargo install cargo-auditable --version 0.7.7 --locked
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo auditable build --release --locked --bin whichride-server \
    && cp target/release/whichride-server /whichride-server

FROM gcr.io/distroless/static-debian13:nonroot
COPY --from=build /whichride-server /usr/local/bin/whichride-server
ENV BIND_ADDR=0.0.0.0:8080 \
    RUST_LOG=info
EXPOSE 8080
USER 65532:65532
ENTRYPOINT ["/usr/local/bin/whichride-server"]
CMD ["serve"]
