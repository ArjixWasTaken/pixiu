# Works with both BuildKit and the legacy builder: dependencies are cached in
# image layers with cargo-chef rather than with BuildKit cache mounts.

# ---- tools -------------------------------------------------------------------
FROM rust:1.98.1-trixie AS chef
# FFmpeg's headers and libclang, for the bindings píxiū remuxes and
# transcodes audio with.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        clang libclang-dev pkg-config libavcodec-dev libavformat-dev libavutil-dev \
        libswresample-dev \
    && rm -rf /var/lib/apt/lists/*
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

# ---- helpers -----------------------------------------------------------------
# yt-dlp (the download fallback) and rustypipe-botguard (optional; see
# pixiu.example.toml), pinned by checksum. Bump versions and hashes together.
FROM debian:trixie-slim AS helpers
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl xz-utils \
    && rm -rf /var/lib/apt/lists/*
RUN set -eux; \
    case "$(dpkg --print-architecture)" in \
        amd64) arch=x86_64; ytdlp=yt-dlp_linux; \
            ytdlp_sha=58162f9bfdc27458ea47bfcb311cf47028f17d8154a8bf7d689861d46399230a; \
            botguard_sha=4f2ec561e8f9fadece7deadc6ce0624fbdedd852222c3eb194c22153b1323129 ;; \
        arm64) arch=aarch64; ytdlp=yt-dlp_linux_aarch64; \
            ytdlp_sha=b16e4dab368a816cd05d477d698a605a6ae87ccee1c8ffd38fa21d7254141fcc; \
            botguard_sha=4d038857374a69aea9be8ded981d93a776dc88d4e254f5c6d292746099abf69a ;; \
        *) echo "no helper binaries for $(dpkg --print-architecture)" >&2; exit 1 ;; \
    esac; \
    curl -fsSL -o /usr/local/bin/yt-dlp \
        "https://github.com/yt-dlp/yt-dlp/releases/download/2026.08.19/${ytdlp}"; \
    echo "${ytdlp_sha}  /usr/local/bin/yt-dlp" | sha256sum -c -; \
    curl -fsSL -o /tmp/botguard.tar.xz \
        "https://codeberg.org/ThetaDev/rustypipe-botguard/releases/download/v0.1.2/rustypipe-botguard-v0.1.2-${arch}-unknown-linux-gnu.tar.xz"; \
    echo "${botguard_sha}  /tmp/botguard.tar.xz" | sha256sum -c -; \
    tar -xJf /tmp/botguard.tar.xz -C /usr/local/bin rustypipe-botguard; \
    chmod 0755 /usr/local/bin/yt-dlp /usr/local/bin/rustypipe-botguard

# yt-dlp needs a JavaScript runtime to answer YouTube's challenges.
FROM denoland/deno:bin-2.9.7 AS deno

# ---- runtime -----------------------------------------------------------------
FROM debian:trixie-slim

# Chromium runs the login browser; the FFmpeg libraries remux downloads
# and transcode streams. Mesa's GPU drivers (and the LLVM they compile
# shaders with, ~190 MB) come along with Chromium but are only used on real
# GPUs; headless Chromium draws with its own SwiftShader, so they go.
RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        ca-certificates chromium fonts-liberation tini \
        libavcodec61 libavformat61 libavutil59 libswresample5 \
    && dpkg --purge --force-depends libgl1-mesa-dri mesa-libgallium libllvm19 libz3-4 \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --uid 1000 --user-group --create-home --shell /usr/sbin/nologin pixiu \
    && mkdir -p /data /treasure \
    && chown pixiu:pixiu /data /treasure

COPY --from=helpers /usr/local/bin/yt-dlp /usr/local/bin/rustypipe-botguard /usr/local/bin/
COPY --from=deno /deno /usr/local/bin/deno
# The binary looks for its asset bundle next to itself.
COPY --from=build /src/target/release/pixiu /opt/pixiu/pixiu
COPY --from=build /src/target/release/assets /opt/pixiu/assets

# Chromium's sandbox needs privileges containers rarely grant.
ENV PIXIU_SERVER__HOST=0.0.0.0 \
    PIXIU_SERVER__PORT=4533 \
    PIXIU_PATHS__DATA_DIR=/data \
    PIXIU_PATHS__TREASURE_DIR=/treasure \
    PIXIU_BROWSER__EXECUTABLE=/usr/bin/chromium \
    PIXIU_BROWSER__NO_SANDBOX=true

USER pixiu
# A `pixiu.toml` placed in the data volume is picked up automatically.
WORKDIR /data
VOLUME ["/data", "/treasure"]
EXPOSE 4533

HEALTHCHECK --interval=30s --timeout=10s --start-period=10s \
    CMD ["/opt/pixiu/pixiu", "healthcheck"]

# tini reaps the processes Chromium leaves behind when it closes.
ENTRYPOINT ["/usr/bin/tini", "--", "/opt/pixiu/pixiu"]
