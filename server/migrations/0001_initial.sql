CREATE TABLE admin (id INTEGER PRIMARY KEY CHECK(id = 1), password_hash TEXT NOT NULL);
CREATE TABLE sessions (token_hash TEXT PRIMARY KEY, csrf TEXT NOT NULL, expires_at INTEGER NOT NULL);
CREATE TABLE albums (
 id TEXT PRIMARY KEY, name TEXT NOT NULL, slug TEXT NOT NULL UNIQUE,
 created_at INTEGER NOT NULL, deleted_at INTEGER
);
CREATE TABLE videos (
 id TEXT PRIMARY KEY, token TEXT NOT NULL UNIQUE, album_id TEXT REFERENCES albums(id),
 name TEXT NOT NULL, visibility TEXT NOT NULL CHECK(visibility IN ('private','public')),
 state TEXT NOT NULL CHECK(state IN ('uploading','verifying','ready','publishing','published','revoking','deleting','deleted','failed')),
 size INTEGER NOT NULL, duration REAL NOT NULL DEFAULT 0, width INTEGER NOT NULL DEFAULT 0,
 height INTEGER NOT NULL DEFAULT 0, poster INTEGER NOT NULL DEFAULT 0,
 created_at INTEGER NOT NULL, revision INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX videos_album ON videos(album_id, created_at DESC);
CREATE TABLE uploads (
 id TEXT PRIMARY KEY, video_id TEXT NOT NULL UNIQUE REFERENCES videos(id),
 upload_id TEXT NOT NULL, expires_at INTEGER NOT NULL, state TEXT NOT NULL DEFAULT 'uploading'
);
CREATE TABLE jobs (
 id TEXT PRIMARY KEY, video_id TEXT NOT NULL REFERENCES videos(id), kind TEXT NOT NULL,
 revision INTEGER NOT NULL, state TEXT NOT NULL DEFAULT 'pending',
 invalidation_id TEXT, attempts INTEGER NOT NULL DEFAULT 0, next_at INTEGER NOT NULL DEFAULT 0,
 error TEXT, created_at INTEGER NOT NULL
);
CREATE INDEX jobs_pending ON jobs(state, next_at);
