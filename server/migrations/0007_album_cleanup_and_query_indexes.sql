-- Deleted albums have been physically removed since migration 0005.
ALTER TABLE albums DROP COLUMN deleted_at;

-- Match the stable order used by paginated administrator and public queries.
CREATE INDEX videos_library_order
    ON videos (created_at DESC, id DESC)
    WHERE state != 'deleted';

CREATE INDEX videos_public_album_order
    ON videos (album_id, created_at DESC, id DESC)
    WHERE visibility = 'public' AND state = 'published';
