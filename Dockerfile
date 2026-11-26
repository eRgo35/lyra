# syntax=docker/dockerfile:1
#
# Lyra with everything it needs at runtime: yt-dlp (kept up to date on start), deno
# (yt-dlp's JavaScript runtime for YouTube) and ffmpeg. See README "Running on a server".

ARG RUST_VERSION=1.100

FROM rust:${RUST_VERSION}-bookworm AS build
# cmake builds the bundled libopus.
RUN apt-get update \
    && apt-get install -y --no-install-recommends cmake \
    && rm -rf /var/lib/apt/lists/*
WORKDIR /src
COPY . .
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/usr/local/cargo/git \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked \
    && cp target/release/lyra /usr/local/bin/lyra

FROM debian:bookworm-slim AS tools
ARG TARGETARCH
ARG YTDLP_VERSION=2026.10.26
ARG YTDLP_SHA256_AMD64=767ba9dce7052aa196f8b12652f433d76de128ba168966fa84b6394d5e6c6795
ARG YTDLP_SHA256_ARM64=e87e9533e89fe5889abaa21f5114dfe7756d6aa6219b83551add04d48b18e483
ARG DENO_VERSION=2.9.8
ARG DENO_SHA256_AMD64=e890975bc2549807bbcc0b584ffafda72cc7c93349dd99c80483dd784f6c4609
ARG DENO_SHA256_ARM64=d49596c20b9788cf8f5d8ddf644103d4bde56ed239753f0bdb92ba9361d51905
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates curl unzip \
    && rm -rf /var/lib/apt/lists/*
RUN set -eu; \
    case "${TARGETARCH:-amd64}" in \
      amd64) ytdlp=yt-dlp_linux; ytdlp_sha=$YTDLP_SHA256_AMD64; deno=x86_64; deno_sha=$DENO_SHA256_AMD64 ;; \
      arm64) ytdlp=yt-dlp_linux_aarch64; ytdlp_sha=$YTDLP_SHA256_ARM64; deno=aarch64; deno_sha=$DENO_SHA256_ARM64 ;; \
      *) echo "unsupported architecture: $TARGETARCH" >&2; exit 1 ;; \
    esac; \
    curl -fsSLo /yt-dlp "https://github.com/yt-dlp/yt-dlp/releases/download/${YTDLP_VERSION}/${ytdlp}"; \
    echo "${ytdlp_sha}  /yt-dlp" | sha256sum -c -; \
    curl -fsSLo /deno.zip "https://github.com/denoland/deno/releases/download/v${DENO_VERSION}/deno-${deno}-unknown-linux-gnu.zip"; \
    echo "${deno_sha}  /deno.zip" | sha256sum -c -; \
    unzip -q /deno.zip -d /; \
    chmod 0755 /yt-dlp /deno

FROM debian:bookworm-slim
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates ffmpeg \
    && rm -rf /var/lib/apt/lists/* \
    && useradd --system --uid 10001 --create-home --home-dir /home/lyra lyra
# Soundboard sounds live in /home/lyra/data (a volume in compose.yaml).
RUN install -d -o lyra -g lyra /home/lyra/data
COPY --from=build /usr/local/bin/lyra /usr/local/bin/lyra
COPY --from=tools /deno /usr/local/bin/deno
# Owned by lyra so `yt-dlp -U` in the entrypoint can replace it.
COPY --from=tools --chown=lyra:lyra /yt-dlp /opt/yt-dlp/yt-dlp
COPY docker/entrypoint.sh /usr/local/bin/lyra-entrypoint
RUN chown lyra:lyra /opt/yt-dlp

# yt-dlp's cache (YouTube player code) in the volume, so it survives restarts.
ENV PATH="/opt/yt-dlp:${PATH}" \
    XDG_CACHE_HOME=/home/lyra/data/cache \
    RUST_LOG=info
USER lyra
WORKDIR /home/lyra
# Lyra shuts down cleanly on Ctrl-C (SIGINT).
STOPSIGNAL SIGINT
ENTRYPOINT ["/usr/local/bin/lyra-entrypoint"]
