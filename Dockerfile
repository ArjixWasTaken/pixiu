# Works with both BuildKit and the legacy builder: dependencies are cached in
# image layers with cargo-chef rather than with BuildKit cache mounts.

# ---- tools -------------------------------------------------------------------
FROM rust:1.98.1-trixie AS chef
# cargo-chef caches dependency builds; the Topcoat CLI bundles the WebUI
# assets (stylesheet, fonts, images).
RUN cargo install cargo-chef@0.1.78 topcoat-cli@0.9.0 --locked
WORKDIR /src

# ---- dependency recipe -------------------------------------------------------
FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ---- build -------------------------------------------------------------------
FROM chef AS build
COPY --from=planner /src/recipe.json recipe.json
# Rebuilt only when dependencies change.
RUN cargo chef cook --release --recipe-path recipe.json

COPY . .
# Builds the binary, then scans it for assets and bundles them next to it.
RUN topcoat asset bundle --release --package pixiu

# ---- runtime -----------------------------------------------------------------
FROM debian:trixie-slim

RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 1000 --user-group --no-create-home --shell /usr/sbin/nologin pixiu \
    && mkdir -p /data /treasure \
    && chown pixiu:pixiu /data /treasure

# The binary looks for its asset bundle next to itself.
COPY --from=build /src/target/release/pixiu /opt/pixiu/pixiu
COPY --from=build /src/target/release/assets /opt/pixiu/assets

ENV PIXIU_SERVER__HOST=0.0.0.0 \
    PIXIU_SERVER__PORT=4533 \
    PIXIU_PATHS__DATA_DIR=/data \
    PIXIU_PATHS__TREASURE_DIR=/treasure

USER pixiu
# A `pixiu.toml` placed in the data volume is picked up automatically.
WORKDIR /data
VOLUME ["/data", "/treasure"]
EXPOSE 4533

HEALTHCHECK --interval=30s --timeout=10s --start-period=10s \
    CMD ["/opt/pixiu/pixiu", "healthcheck"]

ENTRYPOINT ["/opt/pixiu/pixiu"]
