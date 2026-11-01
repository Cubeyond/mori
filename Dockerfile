# Build static assets off the small runtime instance.
FROM node:24-bookworm-slim AS web
COPY --from=ghcr.io/pnpm/pnpm:12.16.1 /opt/pnpm /opt/pnpm
ENV PATH="/opt/pnpm:$PATH"
WORKDIR /src/web
COPY web/package.json web/pnpm-lock.yaml web/pnpm-workspace.yaml ./
COPY web/scripts/api/package.json ./scripts/api/
RUN --mount=type=cache,target=/pnpm/store pnpm install --frozen-lockfile --store-dir /pnpm/store
COPY web/ ./
RUN pnpm run build

# Compile a portable optimized binary using the project release profile.
FROM rust:1.99.0-bookworm AS server
WORKDIR /src
COPY server/ ./server/
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/src/server/target \
    cargo build --manifest-path server/Cargo.toml --release --locked -j 1 && cp server/target/release/mori /usr/local/bin/mori

# Ship no Node runtime, compiler, or video processing tools.
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates && rm -rf /var/lib/apt/lists/* && useradd --uid 10001 --create-home mori && mkdir /data && chown mori:mori /data
COPY --from=server /usr/local/bin/mori /usr/local/bin/mori
COPY --from=web /src/web/build /app/web
USER 10001:10001
ENV MORI_DATABASE_URL=sqlite:///data/mori.db MORI_WEB_DIR=/app/web RUST_LOG=warn
EXPOSE 3000
ENTRYPOINT ["mori"]
CMD ["serve"]
