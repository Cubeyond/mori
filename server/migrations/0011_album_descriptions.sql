-- Optional descriptions preserve existing albums without placeholder copy.
ALTER TABLE albums ADD COLUMN description TEXT NOT NULL DEFAULT ''
    CHECK(length(description) <= 500);

-- Each album cover uses at most three completed previews in stable newest-first order.
CREATE INDEX videos_album_previews
    ON videos (album_id, created_at DESC, id DESC)
    WHERE poster != 0 AND state IN ('ready', 'published');
