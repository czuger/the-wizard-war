-- The rules' phase (or turn start/end) each log line belongs to; NULL for older lines.
ALTER TABLE event_log ADD COLUMN section TEXT;
