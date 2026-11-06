# Build static assets off the small runtime instance.
FROM node:24-bookworm-slim AS web
COPY --from=ghcr.io/pnpm/pnpm:12.16.1 /opt/pnpm /opt/pnpm
ENV PATH="/opt/pnpm:$PATH"
WORKDIR /src/web
COPY web/package.json web/pnpm-lock.yaml web/pnpm-workspace.yaml ./
COPY web/scripts/api/package.json ./scripts/api/
COPY web/patches/ ./patches/
RUN --mount=type=cache,target=/pnpm/store pnpm install --frozen-lockfile --store-dir /pnpm/store
COPY web/ ./
COPY LICENSE NOTICE /src/
RUN pnpm run build

# Compile a portable optimized binary using the project release profile.
FROM rust:1.99.0-bookworm AS server
WORKDIR /src
COPY server/ ./server/
RUN --mount=type=cache,target=/usr/local/cargo/registry --mount=type=cache,target=/src/server/target \
    cargo build --manifest-path server/Cargo.toml --release --locked && cp server/target/release/mori /usr/local/bin/mori
COPY licenses/ ./licenses/
RUN sha256sum --check licenses/cargo.sha256

# Collect the native executable's runtime dependencies and HTTPS trust store.
RUN apt-get update && apt-get install -y --no-install-recommends --only-upgrade \
    libc6 libgcc-s1 ca-certificates && rm -rf /var/lib/apt/lists/*

RUN set -eu; \
    mkdir -p /runtime/data /runtime/tmp /runtime/etc/ssl/certs /runtime/var/lib/dpkg /runtime/usr/share/licenses/mori; \
    gzip -n -9 -c licenses/cargo.json > /runtime/usr/share/licenses/mori/rust.json.gz; \
    LC_ALL=C ldd /usr/local/bin/mori > /tmp/mori-libraries; \
    if grep -q 'not found' /tmp/mori-libraries; then exit 1; fi; \
    awk '$2 == "=>" && $3 ~ /^\// { print $3 } $1 ~ /^\// { print $1 }' /tmp/mori-libraries > /tmp/mori-library-paths; \
    while IFS= read -r library; do cp --parents "$library" /runtime; done < /tmp/mori-library-paths; \
    for library in /lib/*/libnss_dns.so.2 /lib/*/libnss_files.so.2; do \
        if [ -f "$library" ]; then cp --parents "$library" /runtime; fi; \
    done; \
    cp /etc/ssl/certs/ca-certificates.crt /runtime/etc/ssl/certs/; \
    cp -L /etc/os-release /runtime/etc/os-release; \
    cp /etc/debian_version /runtime/etc/debian_version; \
    dpkg-query --status libc6 libgcc-s1 ca-certificates > /runtime/var/lib/dpkg/status; \
    dpkg-query -W -f='${binary:Package}\t${source:Package}\t${source:Version}\n' libc6 libgcc-s1 ca-certificates \
        | awk -F '\t' '{v=$3; gsub(/\+/, "%2B", v); printf "%s: https://sources.debian.org/src/%s/%s/\n", $1, $2, v}' \
        > /runtime/usr/share/licenses/mori/system-sources.txt; \
    for package in libc6 libgcc-s1 ca-certificates; do \
        cp --parents "/usr/share/doc/$package/copyright" /runtime; \
    done; \
    cp -a /usr/share/common-licenses /runtime/usr/share/; \
    printf 'hosts: files dns\npasswd: files\ngroup: files\n' > /runtime/etc/nsswitch.conf; \
    printf 'mori:x:10001:10001:Mori:/data:/sbin/nologin\n' > /runtime/etc/passwd; \
    printf 'mori:x:10001:\n' > /runtime/etc/group; \
    chmod 1777 /runtime/tmp; \
    chmod 700 /runtime/data; \
    chown 10001:10001 /runtime/data

# Attach a version-specific source offer without invalidating the compilation layer.
ARG MORI_SOURCE_URL=https://github.com/Cubeyond/mori
RUN printf '%s\n' "$MORI_SOURCE_URL" > /runtime/usr/share/licenses/mori/SOURCE

# Ship only the executable, its shared libraries, certificates, and static UI.
FROM scratch
LABEL org.opencontainers.image.title="Mori" \
    org.opencontainers.image.licenses="AGPL-3.0-or-later" \
    org.opencontainers.image.source="https://github.com/Cubeyond/mori"
COPY --from=server /runtime/ /
COPY --from=server /usr/local/bin/mori /usr/local/bin/mori
COPY --from=web /src/web/build /app/web
COPY LICENSE NOTICE /usr/share/licenses/mori/
COPY --from=web /src/web/build/THIRD-PARTY-NOTICES.json /usr/share/licenses/mori/web.json
COPY --from=web /src/web/src/lib/components/ui/LICENSE.md /usr/share/licenses/mori/shadcn-svelte.txt
COPY --from=web /src/web/build/themes-LICENSE.txt /usr/share/licenses/mori/themes.txt
COPY --from=web /src/web/build/fonts/OFL.txt /src/web/build/fonts/geist-LICENSE.txt /usr/share/licenses/mori/fonts/
USER 10001:10001
ENV MORI_DATABASE_URL=sqlite:///data/mori.db MORI_WEB_DIR=/app/web RUST_LOG=warn \
    SSL_CERT_FILE=/etc/ssl/certs/ca-certificates.crt
EXPOSE 3000
ENTRYPOINT ["/usr/local/bin/mori"]
CMD ["serve"]
