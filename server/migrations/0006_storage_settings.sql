-- Credentials are authenticated-encrypted with the adjacent, owner-only storage.key file.
CREATE TABLE storage_settings (
 id INTEGER PRIMARY KEY CHECK(id=1),
 ciphertext BLOB NOT NULL,
 updated_at INTEGER NOT NULL
);
