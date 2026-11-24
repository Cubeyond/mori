CREATE TABLE upload_preferences (
    id INTEGER PRIMARY KEY CHECK(id=1),
    optimization TEXT NOT NULL CHECK(optimization IN ('original','balanced','quality'))
);

CREATE TABLE icloud_connection (
    id INTEGER PRIMARY KEY CHECK(id=1),
    generation TEXT NOT NULL,
    account TEXT NOT NULL,
    credentials BLOB NOT NULL,
    needs_auth INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE icloud_schedule (
    id INTEGER PRIMARY KEY CHECK(id=1),
    automatic INTEGER NOT NULL DEFAULT 0,
    frequency TEXT NOT NULL DEFAULT 'daily',
    local_time TEXT NOT NULL DEFAULT '03:00',
    timezone TEXT NOT NULL DEFAULT 'UTC',
    optimization TEXT,
    next_sync INTEGER,
    queued_at INTEGER,
    retries INTEGER NOT NULL DEFAULT 0,
    last_sync INTEGER,
    last_error TEXT,
    imported INTEGER NOT NULL DEFAULT 0,
    skipped INTEGER NOT NULL DEFAULT 0,
    failed INTEGER NOT NULL DEFAULT 0
);
INSERT INTO icloud_schedule(id) VALUES(1);

CREATE TABLE icloud_albums (
    source TEXT PRIMARY KEY,
    album_id TEXT NOT NULL REFERENCES albums(id) ON DELETE CASCADE
);

CREATE TABLE icloud_imports (
    account TEXT NOT NULL,
    album_id TEXT NOT NULL REFERENCES albums(id) ON DELETE CASCADE,
    digest TEXT NOT NULL,
    video_id TEXT NOT NULL REFERENCES videos(id),
    upload_id TEXT NOT NULL REFERENCES uploads(id),
    payload_digest TEXT NOT NULL,
    PRIMARY KEY(account, album_id, digest)
);

CREATE TABLE icloud_seen (
    account TEXT NOT NULL,
    album_id TEXT NOT NULL REFERENCES albums(id) ON DELETE CASCADE,
    fingerprint TEXT NOT NULL,
    PRIMARY KEY(account, album_id, fingerprint)
);
