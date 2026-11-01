-- Historical rows are videos and retain their existing poster locations.
ALTER TABLE videos ADD COLUMN kind TEXT NOT NULL DEFAULT 'video' CHECK(kind IN ('image','video'));
ALTER TABLE videos ADD COLUMN poster_key TEXT;
