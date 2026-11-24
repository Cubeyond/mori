# SPDX-FileCopyrightText: 2026 Cubeyond
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Private iCloud reader and media preparation worker."""

import configparser
import contextlib
import datetime as dt
import base64
import hashlib
import json
import math
import os
from pathlib import Path, PurePosixPath
import re
import shutil
import secrets
import socket
import socketserver
import struct
import subprocess
import threading
import time
from urllib.error import HTTPError
from urllib.request import Request, urlopen
from zoneinfo import ZoneInfo, ZoneInfoNotFoundError

WORK = Path(os.environ.get("MORI_ICLOUD_DIR", "/icloud"))
RUNTIME = Path(os.environ.get("MORI_ICLOUD_RUNTIME", "/run/mori-icloud"))
STAGE = WORK / "stage"
MAX_WORK = int(os.environ.get("MORI_ICLOUD_WORK_BYTES", 8 * 1024**3))
MAX_MESSAGE = 32 * 1024**2
FILE_PAGE = 256
WEEKDAYS = ("monday", "tuesday", "wednesday", "thursday", "friday", "saturday", "sunday")
MEDIA_FORMATS = "asf,avi,apng,bmp_pipe,flv,gif,ico,jpeg_pipe,jpegxl_pipe,matroska,webm,mov,mp4,m4a,3gp,3g2,mj2,mpeg,mpegts,ogg,png_pipe,psd_pipe,tiff_pipe,webp_pipe"
REAUTH_TIMEOUT = 600
VIDEO_TIMEOUT = 900
RETAIN_SECONDS = 86400


class Failure(Exception):
    """A sanitized error code returned to the Mori API."""


class Remote:
    """One loopback-only rclone instance and its volatile credentials."""

    def __init__(self, label, config=None, cache_namespace=None):
        with socket.socket() as listener:
            listener.bind(("127.0.0.1", 0))
            self.port = listener.getsockname()[1]
        password = secrets.token_urlsafe(32)
        self.authorization = "Basic " + base64.b64encode(f"mori:{password}".encode()).decode()
        self.cache = WORK / "rclone-cache" / (cache_namespace or label)
        self.path = RUNTIME / f"{label}.conf"
        self.path.unlink(missing_ok=True)
        if config:
            parser = configparser.ConfigParser(interpolation=None)
            parser["photos"] = config
            with self.path.open("x", encoding="utf-8") as file:
                parser.write(file)
        self.process = subprocess.Popen(
            [
                "rclone", "rcd", "--rc-addr", f"127.0.0.1:{self.port}",
                "--config", str(self.path), "--cache-dir", str(self.cache),
                "--checkers", "2", "--transfers", "1",
                "--buffer-size", "4Mi", "--multi-thread-streams", "0",
                "--contimeout", "10s", "--timeout", "30s", "--retries", "2",
                "--low-level-retries", "2", "--log-level", "ERROR",
            ],
            env={**os.environ, "RCLONE_RC_USER": "mori", "RCLONE_RC_PASS": password,
                 "GOMEMLIMIT": "256MiB"},
            stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
        self.used = time.monotonic()
        try:
            for _ in range(40):
                if self.process.poll() is not None:
                    break
                try:
                    self.call("rc/noop", {}, timeout=1)
                    return
                except Failure:
                    time.sleep(0.05)
            raise Failure("unavailable")
        except BaseException:
            self.close()
            raise

    def close(self):
        """Stop the child and remove the unencrypted tmpfs configuration."""
        if self.process.poll() is None:
            self.process.terminate()
            try:
                self.process.wait(timeout=3)
            except subprocess.TimeoutExpired:
                self.process.kill()
                self.process.wait()
        self.path.unlink(missing_ok=True)

    def call(self, operation, arguments, timeout=30):
        """Make a fixed rclone RC operation without exposing its raw errors."""
        self.used = time.monotonic()
        request = Request(
            f"http://127.0.0.1:{self.port}/{operation}",
            json.dumps(arguments, ensure_ascii=False).encode(),
            {"Content-Type": "application/json", "Authorization": self.authorization},
        )
        try:
            with urlopen(request, timeout=timeout) as response:
                data = response.read(MAX_MESSAGE + 1)
                if len(data) > MAX_MESSAGE:
                    raise Failure("unavailable")
                return json.loads(data)
        except HTTPError as error:
            # The upstream response can include Apple identifiers or signed URLs.
            message = error.read(8192).decode("utf-8", errors="replace").lower()
            if any(part in message for part in ("requestpcs", "pcs cookies", "consent required", "web access")):
                raise Failure("approval") from None
            if any(part in message for part in ("trust token expired", "missing icloud trust token", "invalid session", "reauth", "empty password", "password is required", "password required")):
                raise Failure("auth") from None
            if not operation.startswith("config/") and any(
                part in message for part in ("srp authentication failed", "invalid account or password", "401 unauthorized")
            ):
                raise Failure("auth") from None
            if operation.startswith("config/") and any(
                part in message for part in ("invalid", "incorrect", "verification", "password", "authentication failed")
            ):
                raise Failure("credentials") from None
            if "object not found" in message or "directory not found" in message:
                raise Failure("missing") from None
            raise Failure("unavailable") from None
        except (OSError, ValueError, TimeoutError):
            raise Failure("unavailable") from None


class Connector:
    """Hold one pending Apple challenge and one active connection."""

    def __init__(self):
        self.pending = None
        self.active = None
        self.generation = None
        self.saved_session = None
        self.inventory = None
        self.last_cleanup = 0

    def discard_challenge(self):
        """Erase the Apple password when its short-lived challenge ends."""
        if self.pending:
            self.pending["remote"].close()
            self.pending = None

    def forget(self):
        """Drop cached sessions when the administrator disconnects."""
        self.discard_challenge()
        if self.active:
            self.active.close()
            self.active = None
        self.generation = None
        self.saved_session = None
        self.inventory = None
        shutil.rmtree(WORK / "rclone-cache", ignore_errors=True)
        for work in STAGE.iterdir():
            if work.is_dir():
                shutil.rmtree(work, ignore_errors=True)

    def expire(self):
        """Release idle processes while keeping on-disk album indexes."""
        now = time.monotonic()
        if now - self.last_cleanup > 300:
            prune_spool()
            self.last_cleanup = now
        if self.pending and self.pending["expires"] < now:
            self.discard_challenge()
        if self.active and now - self.active.used > 120:
            self.active.close()
            self.active = None
        if self.inventory and now - self.inventory[2] > 1800:
            self.inventory = None
        if self.active and self.active.process.poll() is not None:
            self.active.close()
            self.active = None

    def challenge_result(self, reply, retry=False):
        """Project provider questions into Mori's limited verification states."""
        pending = self.pending
        option = reply.get("Option")
        state = reply.get("State") or ""
        if not state and not option:
            remote = pending["remote"]
            try:
                data = remote.call("config/get", {"name": "pending"})
                session = {key: data[key] for key in ("cookies", "trust_token")}
                if not session["trust_token"] or not session["cookies"]:
                    raise Failure("auth")
                return {"state": "connected", "account": pending["account"], "session": session}
            finally:
                self.discard_challenge()
        if option:
            kind = {"config_2fa": "code", "config_2fa_phone": "phone", "config_2fa_sms": "sms"}.get(option.get("Name"))
            if not kind:
                self.discard_challenge()
                raise Failure("unavailable")
            pending["state"] = state
            pending["kind"] = kind
            pending["phones"] = [
                {"value": str(item["Value"]), "label": item.get("Help") or "Trusted phone"}
                for item in (option.get("Examples") or [])
            ] if kind == "phone" else []
        return {
            "state": "challenge", "id": pending["id"], "kind": pending["kind"],
            "phones": pending["phones"], "retry": retry or bool(reply.get("Error")),
        }

    def authenticate(self, request):
        """Drive rclone's resumable config protocol without putting secrets in argv."""
        operation = request["op"]
        if operation == "start":
            self.discard_challenge()
            remote = Remote("auth")
            self.pending = {
                "id": request["challenge"], "account": request["account"], "remote": remote,
                "expires": time.monotonic() + REAUTH_TIMEOUT,
            }
            try:
                return self.challenge_result(remote.call(
                    "config/create",
                    {
                        "name": "pending", "type": "iclouddrive",
                        "parameters": {"service": "photos", "apple_id": request["account"], "password": request["password"]},
                        "opt": {"nonInteractive": True, "obscure": True},
                    },
                ))
            except BaseException:
                self.discard_challenge()
                raise

        pending = self.pending
        if not pending or pending["id"] != request["challenge"]:
            raise Failure("expired")
        if operation == "cancel":
            self.discard_challenge()
            return {"state": "canceled"}
        reply = request["answer"]
        if pending["kind"] == "phone":
            accepted = reply in [item["value"] for item in pending["phones"]]
        else:
            accepted = bool(re.fullmatch(r"[0-9]{6}", reply)) or (pending["kind"] == "code" and reply == "sms")
        if not accepted:
            return self.challenge_result({"State": pending["state"]}, retry=True)
        try:
            result = pending["remote"].call(
                "config/update", {"name": "pending", "parameters": {}, "opt": {
                    "nonInteractive": True, "continue": True, "state": pending["state"], "result": reply,
                }},
            )
        except Failure as error:
            if str(error) != "credentials":
                raise
            return self.challenge_result({"State": pending["state"]}, retry=True)
        return self.challenge_result(result)

    def reader(self, connection):
        """Reopen the saved cookie session when a run or worker restarts."""
        generation = connection["generation"]
        if self.active and self.generation != generation:
            self.active.close()
            self.active = None
        if not self.active:
            if self.generation != generation:
                for work in STAGE.iterdir():
                    if work.is_dir() and time.time() - work.stat().st_mtime > RETAIN_SECONDS:
                        shutil.rmtree(work, ignore_errors=True)
            namespace = hashlib.sha256(connection["account"].encode()).hexdigest()
            self.active = Remote("photos", {
                "type": "iclouddrive", "service": "photos", "apple_id": connection["account"],
                **connection["session"],
                # Keep slash-containing album or asset names inside each path component.
                "encoding": "Slash,BackSlash,Del,Ctl,InvalidUtf8,Dot",
            }, cache_namespace=namespace)
            self.generation = generation
            self.saved_session = connection["session"]
        return self.active

    def response(self, value):
        """Return refreshed cookies only when rclone changes the private session."""
        config = self.active.call("config/get", {"name": "photos"})
        session = {key: config.get(key, "") for key in ("cookies", "trust_token")}
        changed = None
        if session["cookies"] and session["trust_token"] and session != self.saved_session:
            self.saved_session = session
            changed = session
        return {"value": value, "session": changed}

    def files(self, remote, request):
        """Keep large album inventories in the worker and send small pages to Mori."""
        path, offset = request["album"], request["offset"]
        key = (self.generation, path)
        if offset == 0 or not self.inventory or self.inventory[0] != key:
            items = [
                {"path": item["Path"], "size": item["Size"], "modified": item.get("ModTime") or ""}
                for item in list_remote(remote, path, {"filesOnly": True}, timeout=600)
                if not item.get("IsDir") and valid_path(item["Path"])
            ]
        else:
            items = self.inventory[1]
        end = offset + FILE_PAGE
        more = end < len(items)
        self.inventory = (key, items, time.monotonic()) if more else None
        return {"assets": items[offset:end], "more": more}

    def dispatch(self, request):
        """Run one private operation. All processing shares a single worker slot."""
        self.expire()
        operation = request["op"]
        if operation in ("start", "answer", "cancel"):
            return self.authenticate(request)
        if operation == "forget":
            self.forget()
            return {}
        if operation == "remove":
            shutil.rmtree(stage_path(request["stage"]), ignore_errors=True)
            return {}
        remote = self.reader(request["connection"])
        if operation == "albums":
            return self.response(albums(remote))
        if operation == "files":
            return self.response(self.files(remote, request))
        if operation == "prepare":
            return self.response(prepare(remote, request))
        raise Failure("unavailable")


def valid_path(path):
    """Allow provider paths while keeping traversal and control characters out."""
    return (
        isinstance(path, str) and 0 < len(path) <= 4096
        and all(ord(char) >= 32 and ord(char) != 127 for char in path)
        and all(part not in ("", ".", "..") for part in path.split("/"))
    )


def stage_path(key):
    """Resolve a generated stage identity, never an upstream filename."""
    if not re.fullmatch(r"[a-f0-9]{64}", key):
        raise Failure("unavailable")
    return STAGE / key


def list_remote(remote, path, options, timeout=120):
    """Read a bounded list from the named Photos remote only."""
    if path and not valid_path(path):
        raise Failure("unavailable")
    items = remote.call("operations/list", {
        "fs": "photos:", "remote": path, "opt": options,
    }, timeout=timeout)["list"] or []
    if path:
        for item in items:
            item["Path"] = item["Path"].removeprefix(path + "/")
    return items


def albums(remote):
    """Discover nested album paths without downloading media payloads."""
    results = []
    for library in list_remote(remote, "", {"dirsOnly": True}):
        name = library["Name"]
        if not valid_path(name) or (name != "PrimarySync" and not name.startswith("SharedSync-")):
            continue
        paths = library_albums(remote, name)
        for path in sorted(paths):
            display = path if name == "PrimarySync" else f"{name}/{path}"
            results.append({"path": f"{name}/{path}", "name": display})
    return results


def library_albums(remote, library):
    """Use rclone's album cache to descend into folders without scanning media."""
    options = {"dirsOnly": True, "noMimeType": True, "noModTime": True}
    entries = list_remote(remote, library, options)
    cache = remote.cache / "iclouddrive-photos" / "photos" / library / "albums.json"
    try:
        with cache.open("rb") as file:
            cached = json.loads(file.read(MAX_MESSAGE))
        return album_leaves(remote, library, "", options, entries, cached)
    except (OSError, TypeError, ValueError, AttributeError):
        # Keep the public rclone listing as a fallback if the cache is absent
        # or provider names do not match its directory representation.
        entries = list_remote(remote, library, {**options, "recurse": True})
        paths = {item["Path"] for item in entries if item.get("IsDir") and valid_path(item["Path"])}
        parents = {str(PurePosixPath(path).parent) for path in paths}
        return paths - parents


def album_leaves(remote, library, prefix, options, entries, cached):
    """Take path names from the live listing and folder types from its cache."""
    folders = {name: album for name, album in cached.items() if album.get("isFolder")}
    names = {item["Path"] for item in entries if item.get("IsDir") and valid_path(item["Path"])}
    if not folders.keys() <= names:
        raise ValueError("Unmatched iCloud folder")
    leaves = {prefix + name for name in names - folders.keys()}
    for name, folder in folders.items():
        path = prefix + name
        children = list_remote(remote, f"{library}/{path}", options)
        leaves.update(album_leaves(remote, library, path + "/", options, children, folder.get("children", {})))
    return leaves


def next_sync(request):
    """Resolve the next local calendar slot, skipping gaps and choosing the first fold."""
    try:
        if not re.fullmatch(r"(?:[01][0-9]|2[0-3]):[0-5][0-9]", request["time"]):
            raise ValueError
        zone = ZoneInfo(request["timezone"])
        after = request["after"]
        frequency = request["frequency"]
        interval = {"hourly": 3600, "six_hours": 6 * 3600}.get(frequency)
        if interval:
            return after + interval
        current = dt.datetime.fromtimestamp(after, zone)
        if frequency == "weekly":
            first_day = (WEEKDAYS.index(request["weekday"]) - current.weekday()) % 7
            period = 7
        elif frequency == "daily":
            first_day, period = 0, 1
        else:
            raise ValueError
        local_time = dt.time.fromisoformat(request["time"])
        for days in range(first_day, first_day + 3 * period, period):
            local = dt.datetime.combine(current.date() + dt.timedelta(days=days), local_time)
            timestamp = int(local.replace(tzinfo=zone, fold=0).timestamp())
            if timestamp > after and dt.datetime.fromtimestamp(timestamp, zone).replace(tzinfo=None) == local:
                return timestamp
        raise ValueError
    except (KeyError, ValueError, ZoneInfoNotFoundError):
        raise Failure("schedule") from None


def disk_budget(work, size):
    """Keep enough memory and disk for the service and the current spool."""
    if size + 4 * 1024**2 > MAX_WORK:
        raise Failure("too_large")
    try:
        memory = next(int(line.split()[1]) for line in Path("/proc/meminfo").read_text().splitlines() if line.startswith("MemAvailable:"))
        processors = sum(line.startswith("cpu") and line[3:4].isdigit() for line in Path("/proc/stat").read_text().splitlines())
        if memory < 384 * 1024 or os.getloadavg()[0] >= max(1, processors):
            raise Failure("resources")
        prune_spool(work)
        retained = sum(path.stat().st_size for path in STAGE.rglob("*") if path.is_file())
        available = min(MAX_WORK - retained, shutil.disk_usage(STAGE).free - 256 * 1024**2)
        if size + 4 * 1024**2 > available:
            raise Failure("resources")
        return available - size - 4 * 1024**2
    except (OSError, StopIteration):
        raise Failure("unavailable") from None


def prune_spool(active=None):
    """Remove expired retry payloads even while the connector is idle."""
    with contextlib.suppress(OSError):
        for entry in STAGE.iterdir():
            if entry.is_dir() and entry != active and time.time() - entry.stat().st_mtime > RETAIN_SECONDS:
                shutil.rmtree(entry, ignore_errors=True)


def read_source(remote, request, work):
    """Download exactly the selected asset into a deterministic local spool."""
    album, asset = request["album"], request["file"]
    if not valid_path(album) or not valid_path(asset):
        raise Failure("unavailable")
    source = work / "source"
    try:
        remote.call("operations/copyfile", {
            "srcFs": "photos:", "srcRemote": f"{album}/{asset}",
            "dstFs": str(work), "dstRemote": source.name,
        }, timeout=900)
    except Failure as error:
        if str(error) != "missing":
            raise
        # A Photos listing can expose a disambiguated name that direct lookup
        # cannot resolve. Copying that exact listed entry bypasses this lookup.
        fallback = work / "listed"
        fallback.mkdir()
        glob = re.sub(r"([\\*?\[\]{}])", r"\\\1", asset)
        remote.call("sync/copy", {
            "srcFs": f"photos:{album}", "dstFs": str(fallback),
            "_filter": {"IncludeRule": [f"/{glob}"]},
        }, timeout=900)
        files = [entry for entry in fallback.rglob("*") if entry.is_file()]
        if len(files) != 1:
            raise Failure("unavailable")
        files[0].replace(source)
        shutil.rmtree(fallback)
    size = source.stat().st_size
    if size < 16 or size > request["max"]:
        raise Failure("too_large")
    if request["size"] > 0 and size != request["size"]:
        raise Failure("unavailable")
    return source


def get_probe(path):
    """Return bounded stream metadata from the downloaded file."""
    args = [
        "ffprobe", "-v", "error", "-threads", "1", "-protocol_whitelist", "file",
        "-format_whitelist", MEDIA_FORMATS,
        "-show_entries", "format=duration:stream=index,codec_type,codec_name,width,height,pix_fmt,color_transfer,color_primaries,color_space,nb_frames,sample_aspect_ratio:stream_disposition=attached_pic:stream_side_data=rotation",
        "-of", "json", str(path),
    ]
    try:
        result = subprocess.run(args, stdin=subprocess.DEVNULL, stdout=subprocess.PIPE,
                                stderr=subprocess.DEVNULL, timeout=15, check=True)
        payload = json.loads(result.stdout)
        return payload.get("streams", []), duration(payload.get("format", {}))
    except (subprocess.SubprocessError, OSError, ValueError):
        return [], 0.0


def duration(data):
    """Validate a provider duration before returning it to Mori."""
    try:
        value = float(data.get("duration", 0))
        return value if math.isfinite(value) and 0 < value <= 31_536_000 else 0.0
    except (TypeError, ValueError):
        return 0.0


def dimensions(video):
    """Return only dimensions accepted by Mori's normal upload policy."""
    return tuple(min(32768, max(0, int(video.get(name, 0)))) for name in ("width", "height"))


def compressible(request, streams, track, width, height):
    """Keep multitrack, HDR, rotated and unrecognized media in the original form."""
    if request["optimization"] == "original" or not (width and height):
        return False
    if not request["video"]:
        return request["extension"] in ("jpg", "jpeg", "png") and len(streams) == 1 and width * height <= 12_000_000 and (
            track.get("codec_name") in ("mjpeg", "png") and track.get("nb_frames") in (None, "1")
        )
    return (
        request["extension"] in ("mov", "mp4", "m4v", "webm")
        and width * height <= 1920 * 1080
        and track.get("codec_name") in ("h264", "hevc", "vp8", "vp9", "av1")
        and track.get("pix_fmt") in ("yuv420p", "yuvj420p")
        and track.get("color_transfer") in (None, "bt709", "smpte170m", "iec61966-2-1")
        and track.get("color_primaries") in (None, "bt709", "smpte170m", "bt470bg")
        and track.get("sample_aspect_ratio") in (None, "1:1")
        and not any(item.get("rotation") for item in track.get("side_data_list", []))
        and sum(item.get("codec_type") == "video" for item in streams) == 1
        and all(item.get("codec_type") == "video" or item.get("codec_name") == "aac" for item in streams)
    )


def ffmpeg(source, target, options, timeout):
    """Run one bounded command with one decoder, filter and encoder thread."""
    args = [
        "ffmpeg", "-nostdin", "-hide_banner", "-loglevel", "error", "-y",
        "-threads", "1", "-filter_threads", "1", "-filter_complex_threads", "1",
        "-protocol_whitelist", "file", "-format_whitelist", MEDIA_FORMATS, "-i", str(source),
        *options, "-threads", "1", str(target),
    ]
    try:
        subprocess.run(args, stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL,
                       stderr=subprocess.DEVNULL, timeout=timeout, check=True)
        return True
    except (subprocess.SubprocessError, OSError):
        target.unlink(missing_ok=True)
        return False


def optimized(source, work, request, streams, track, width, height, length, spare):
    """Accept only a complete, sufficiently smaller output."""
    size = source.stat().st_size
    extension = request["extension"]
    ceiling = size * 95 // 100
    if ceiling > spare or not compressible(request, streams, track, width, height):
        return source, extension
    target = work / "optimized"
    quality = request["optimization"] == "quality"
    if request["video"]:
        if not length:
            return source, extension
        output = "mp4"
        options = ["-map", "0:v:0", "-map", "0:a?", "-c:v", "libx264", "-preset", "veryfast",
                   "-crf", "19" if quality else "23", "-c:a", "copy", "-movflags", "+faststart", "-f", output]
    else:
        output = "webp"
        options = ["-frames:v", "1", "-c:v", "libwebp", "-quality", "95" if quality else "90", "-f", output]
    if ffmpeg(source, target, [*options, "-fs", str(ceiling)], VIDEO_TIMEOUT):
        output_size = target.stat().st_size
        output_tracks, output_length = get_probe(target)
        if (16 <= output_size < ceiling and output_tracks
                and dimensions(output_tracks[0]) == (width, height)
                and (not request["video"] or abs(output_length - length) < 0.5)):
            return target, output
    target.unlink(missing_ok=True)
    return source, extension


def sha256(path):
    """Hash a disk file in bounded chunks."""
    with path.open("rb") as file:
        return hashlib.file_digest(file, "sha256").hexdigest()


def prepare(remote, request):
    """Download, prepare and persist exactly one resumable upload payload."""
    work = stage_path(request["stage"])
    manifest = work / "manifest.json"
    if manifest.is_file():
        value = json.loads(manifest.read_bytes())
        path = work / value["file"]
        if path.is_file() and path.stat().st_size == value["size"]:
            return value
    shutil.rmtree(work, ignore_errors=True)
    work.mkdir()
    try:
        expected = request["size"] if request["size"] > 0 else request["max"]
        spare = disk_budget(work, expected)
        source = read_source(remote, request, work)
        source_digest = sha256(source)
        streams, length = get_probe(source)
        video = next((stream for stream in streams if stream.get("codec_type") == "video"), {})
        width, height = dimensions(video)
        payload, extension = optimized(source, work, request, streams, video, width, height, length, spare)
        if payload != source:
            source.unlink()
        poster = work / "poster.jpg"
        has_poster = bool(width and height and width * height <= 20_000_000 and ffmpeg(
            payload, poster, ["-map", "0:v:0", "-frames:v", "1", "-vf",
                              "scale=512:512:force_original_aspect_ratio=decrease:force_divisible_by=2",
                              "-q:v", "4", "-f", "image2"], 20,
        ) and 4 <= poster.stat().st_size <= 1_048_576)
        if not has_poster:
            poster.unlink(missing_ok=True)
        value = {
            "file": payload.name, "source_digest": source_digest,
            "payload_digest": source_digest if payload.name == "source" else sha256(payload),
            "extension": extension, "size": payload.stat().st_size,
            "width": width, "height": height, "duration": length if request["video"] else 0,
            "poster": has_poster,
        }
        partial = work / "manifest.tmp"
        partial.write_text(json.dumps(value), encoding="utf-8")
        partial.replace(manifest)
        return value
    except BaseException:
        shutil.rmtree(work, ignore_errors=True)
        raise


class Handler(socketserver.StreamRequestHandler):
    """Handle one capped IPC message, reporting only fixed error codes."""

    def handle(self):
        self.request.settimeout(15)
        try:
            header = self.rfile.read(4)
            (length,) = struct.unpack(">I", header)
            if not 0 < length <= MAX_MESSAGE:
                return
            request = json.loads(self.rfile.read(length))
            if request["op"] == "next":
                result = next_sync(request)
            else:
                with self.server.processing:
                    result = self.server.connector.dispatch(request)
            message = {"result": result}
        except Failure as error:
            message = {"error": str(error)}
        except (KeyError, ValueError, OSError, TypeError, struct.error):
            message = {"error": "unavailable"}
        data = json.dumps(message, ensure_ascii=False, separators=(",", ":")).encode()
        if len(data) > MAX_MESSAGE:
            data = b'{"error":"unavailable"}'
        with contextlib.suppress(OSError):
            self.wfile.write(struct.pack(">I", len(data)) + data)


class Server(socketserver.ThreadingMixIn, socketserver.UnixStreamServer):
    """Calculate schedules independently of the single media processing slot."""

    def __init__(self, path, handler):
        self.processing = threading.Lock()
        super().__init__(path, handler)

    def service_actions(self):
        if self.processing.acquire(blocking=False):
            try:
                self.connector.expire()
            finally:
                self.processing.release()

    def handle_error(self, _request, _client_address):
        # Never print decoded request or an exception containing provider secrets.
        print("iCloud worker request failed", flush=True)


def main():
    """Serve the optional worker on an owner-only Unix socket."""
    os.umask(0o077)
    WORK.mkdir(parents=True, exist_ok=True)
    STAGE.mkdir(exist_ok=True)
    RUNTIME.mkdir(parents=True, exist_ok=True)
    path = WORK / "worker.sock"
    path.unlink(missing_ok=True)
    connector = Connector()
    try:
        with Server(str(path), Handler) as server:
            server.connector = connector
            path.chmod(0o600)
            server.serve_forever()
    finally:
        connector.discard_challenge()
        if connector.active:
            connector.active.close()
        path.unlink(missing_ok=True)


if __name__ == "__main__":
    main()
