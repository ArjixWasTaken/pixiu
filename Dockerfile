# Alpine (musl) for the server. Works with both BuildKit and the legacy
# builder: dependencies are cached in image layers with cargo-chef rather
# than with BuildKit cache mounts.

# ---- web player ----------------------------------------------------------------
# Static files, so any platform builds them; Debian's Node spares the build
# tools' native binaries a musl hunt.
FROM node:26-slim AS web
# The build tools fetch over HTTPS, so they need the CA certificates.
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/* \
    && npm install --global @nubjs/nub@0.9.5
WORKDIR /web
COPY web/package.json web/pnpm-lock.yaml web/pnpm-workspace.yaml ./
RUN nub ci
COPY web/ ./
RUN nub run build

# ---- tools -------------------------------------------------------------------
FROM rust:1.98.1-alpine3.24 AS chef
# FFmpeg's headers and libclang, for the bindings píxiū remuxes and
# transcodes audio with; a C toolchain for SQLite, QuickJS and the TLS
# crates.
RUN apk add --no-cache \
        clang-dev cmake curl ffmpeg-dev linux-headers make musl-dev perl pkgconf
# Link musl dynamically (Rust links it statically by default), so the binary
# can use Alpine's FFmpeg libraries and build scripts can load libclang.
RUN printf '[target.%s-unknown-linux-musl]\nrustflags = ["-C", "target-feature=-crt-static"]\n' \
        "$(uname -m)" > "$CARGO_HOME/config.toml"
# cargo-chef caches dependency builds.
RUN cargo install cargo-chef@0.1.78 --locked
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
RUN cargo build --release --package pixiu

# ---- helpers -----------------------------------------------------------------
# yt-dlp, the download fallback, as its standalone musl build, pinned by
# checksum. Bump the version and hashes together.
FROM alpine:3.24 AS helpers
RUN apk add --no-cache curl
RUN set -eux; \
    case "$(apk --print-arch)" in \
        x86_64) ytdlp=yt-dlp_musllinux; \
            sha=f3dec9cfeaf304cec98290fe41c6ad465d4b747d302473559643e7af24929722 ;; \
        aarch64) ytdlp=yt-dlp_musllinux_aarch64; \
            sha=17b164c4d258be92bb1ad146cb7c336b783aedb380814aabbcb7d52937f77e57 ;; \
        *) echo "no yt-dlp build for $(apk --print-arch)" >&2; exit 1 ;; \
    esac; \
    curl -fsSL -o /usr/local/bin/yt-dlp \
        "https://github.com/yt-dlp/yt-dlp/releases/download/2026.08.19/${ytdlp}"; \
    echo "${sha}  /usr/local/bin/yt-dlp" | sha256sum -c -; \
    chmod 0755 /usr/local/bin/yt-dlp

# ---- runtime -----------------------------------------------------------------
FROM alpine:3.24

# Chromium runs the login browser; the FFmpeg libraries remux downloads and
# transcode streams; Deno is the JavaScript runtime yt-dlp answers YouTube's
# challenges with.
#
# Mesa's GPU drivers (and the LLVM they compile shaders with, ~230 MB) come
# along with Chromium but are only loaded for real GPUs; headless Chromium
# draws in software, so they go, and so does the file-type database that
# came with Chromium's desktop integration.
RUN apk add --no-cache \
        ca-certificates chromium deno font-liberation tini \
        ffmpeg-libavcodec ffmpeg-libavformat ffmpeg-libavutil ffmpeg-libswresample \
    && rm -rf /usr/lib/libLLVM* /usr/lib/llvm[0-9]* /usr/lib/libgallium-* \
        /usr/lib/dri /usr/lib/gbm /usr/bin/spirv-* /usr/lib/libSPIRV-Tools* \
        /usr/share/misc/magic.mgc \
    && adduser -D -u 1000 -h /home/pixiu -s /sbin/nologin pixiu \
    && mkdir -p /data /treasure \
    && chown pixiu:pixiu /data /treasure

COPY --from=helpers /usr/local/bin/yt-dlp /usr/local/bin/
# The binary looks for the web player next to itself.
COPY --from=build /src/target/release/pixiu /opt/pixiu/pixiu
COPY --from=web /web/dist /opt/pixiu/web

# Chromium's sandbox needs privileges containers rarely grant.
ENV PIXIU_SERVER__HOST=0.0.0.0 \
    PIXIU_SERVER__PORT=4533 \
    PIXIU_PATHS__DATA_DIR=/data \
    PIXIU_PATHS__TREASURE_DIR=/treasure \
    PIXIU_BROWSER__EXECUTABLE=/usr/lib/chromium/chromium \
    PIXIU_BROWSER__NO_SANDBOX=true

USER pixiu
# A `pixiu.toml` placed in the data volume is picked up automatically.
WORKDIR /data
VOLUME ["/data", "/treasure"]
EXPOSE 4533

HEALTHCHECK --interval=30s --timeout=10s --start-period=10s \
    CMD ["/opt/pixiu/pixiu", "healthcheck"]

# tini reaps the processes Chromium leaves behind when it closes.
ENTRYPOINT ["/sbin/tini", "--", "/opt/pixiu/pixiu"]
