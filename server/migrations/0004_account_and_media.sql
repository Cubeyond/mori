-- Single-user credentials remain hashed; existing administrator names default to root.
ALTER TABLE admin ADD COLUMN username TEXT NOT NULL DEFAULT 'root';
-- Preserve the original URLs of existing MP4 records while supporting other containers.
ALTER TABLE videos ADD COLUMN extension TEXT NOT NULL DEFAULT 'mp4';
ALTER TABLE videos ADD COLUMN media_prefix TEXT NOT NULL DEFAULT 'file';
UPDATE videos SET media_prefix=COALESCE((SELECT slug FROM albums WHERE albums.id=videos.album_id),'file');
-- Preserve previously shared pretty paths when album membership changes.
CREATE TABLE media_aliases (
 video_id TEXT NOT NULL REFERENCES videos(id), prefix TEXT NOT NULL,
 PRIMARY KEY(video_id,prefix)
);
INSERT INTO media_aliases(video_id,prefix) SELECT id,media_prefix FROM videos;
