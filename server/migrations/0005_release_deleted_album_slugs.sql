-- Release historical deleted albums and remove aliases under their reusable slugs.
DELETE FROM media_aliases WHERE prefix IN (SELECT slug FROM albums WHERE deleted_at IS NOT NULL);
INSERT OR IGNORE INTO media_aliases(video_id,prefix)
 SELECT id,'file' FROM videos WHERE album_id IN (SELECT id FROM albums WHERE deleted_at IS NOT NULL);
UPDATE videos SET album_id=NULL,media_prefix='file'
 WHERE album_id IN (SELECT id FROM albums WHERE deleted_at IS NOT NULL);
DELETE FROM albums WHERE deleted_at IS NOT NULL;
