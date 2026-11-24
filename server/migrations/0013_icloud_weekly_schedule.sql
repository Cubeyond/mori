ALTER TABLE icloud_schedule ADD COLUMN weekday TEXT NOT NULL DEFAULT 'sunday'
    CHECK (weekday IN ('monday','tuesday','wednesday','thursday','friday','saturday','sunday'));

UPDATE icloud_schedule SET frequency='weekly'
WHERE NOT EXISTS(SELECT 1 FROM icloud_connection)
  AND automatic=0 AND next_sync IS NULL AND last_sync IS NULL
  AND frequency='daily' AND local_time='03:00' AND timezone='UTC' AND optimization IS NULL;
