FROM rust:1.99.0-bookworm AS build
WORKDIR /build
ENV CARGO_BUILD_JOBS=2
RUN apt-get update && apt-get install -y --no-install-recommends python3 && rm -rf /var/lib/apt/lists/*
RUN rustup target add wasm32-unknown-unknown && cargo install wasm-bindgen-cli --version 0.2.129 --locked
COPY . .
RUN cargo build --locked --release -p vela-server && scripts/build-web.sh && python3 scripts/generate_notices.py

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/* \
    && groupadd --gid 10001 vela && useradd --uid 10001 --gid vela --no-create-home vela \
    && mkdir -p /data /app && chown vela:vela /data
COPY --from=build /build/target/release/vela-server /app/vela-server
COPY --from=build /build/dist /app/web
COPY --from=build /build/LICENSE /app/LICENSE
USER 10001:10001
ENV VELA_BIND=0.0.0.0:3210 VELA_DATA_DIR=/data VELA_WEB_DIR=/app/web
EXPOSE 3210
VOLUME ["/data"]
HEALTHCHECK --interval=30s --timeout=5s --start-period=10s CMD curl --fail --silent http://127.0.0.1:3210/health || exit 1
STOPSIGNAL SIGTERM
ENTRYPOINT ["/app/vela-server"]
