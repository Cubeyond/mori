# SPDX-FileCopyrightText: 2026 Cubeyond
# SPDX-License-Identifier: AGPL-3.0-or-later

# rclone v1.77.0-beta.10826 includes upstream's Mainland China fix (571b965).
FROM rclone/rclone:beta@sha256:c4843db26f8e82fa383df68b3e25a2bc2b6834cfed6bd718f986b9232d0c2d5a AS rclone
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates ffmpeg python3 tzdata \
    && rm -rf /var/lib/apt/lists/* \
    && install -d -m 700 -o 10001 -g 10001 /icloud \
    && install -m 600 -o 10001 -g 10001 /dev/null /icloud/.worker-volume
COPY --from=rclone /usr/local/bin/rclone /usr/local/bin/rclone
COPY sync/icloud.py /usr/local/lib/mori/icloud.py
COPY LICENSE NOTICE /usr/share/licenses/mori/
USER 10001:10001
ENTRYPOINT ["python3", "-u", "/usr/local/lib/mori/icloud.py"]
