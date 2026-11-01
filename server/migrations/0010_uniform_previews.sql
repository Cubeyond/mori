-- Originals and previews share the same layout for both media categories and visibility states.
-- Keep historical pointers until a verified, retryable object move replaces them.
INSERT INTO jobs(id,video_id,kind,revision,created_at)
SELECT lower(hex(randomblob(16))),id,'normalize',revision,unixepoch()
FROM videos WHERE state IN ('ready','published')
AND (source_key IS NULL OR poster_key IS NULL OR poster_key NOT LIKE 'private/files/posters/%')
AND NOT EXISTS(SELECT 1 FROM jobs j WHERE j.video_id=videos.id AND j.revision=videos.revision
    AND j.kind='normalize' AND j.state='pending');
