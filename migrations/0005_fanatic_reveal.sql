-- Fanatics revealed in phase IV stay face up in the great hall until the end of the turn.
ALTER TABLE fanatics ADD COLUMN revealed INTEGER NOT NULL DEFAULT 0;
