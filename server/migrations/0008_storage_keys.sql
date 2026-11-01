-- Existing object locations remain readable until their durable normalization job succeeds.
ALTER TABLE videos ADD COLUMN source_key TEXT;

INSERT INTO jobs(id,video_id,kind,revision,created_at)
SELECT lower(hex(randomblob(16))),id,'normalize',revision,unixepoch()
FROM videos WHERE state IN ('ready','published');
