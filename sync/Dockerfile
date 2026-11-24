# SPDX-FileCopyrightText: 2026 Cubeyond
# SPDX-License-Identifier: AGPL-3.0-or-later

FROM rclone/rclone:1.76.0 AS rclone
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
