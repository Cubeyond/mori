-- Preserve existing shared collections; new albums default to private.
ALTER TABLE albums ADD COLUMN visibility TEXT NOT NULL DEFAULT 'private'
 CHECK(visibility IN ('private','public'));
UPDATE albums SET visibility='public';
