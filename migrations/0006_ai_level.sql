-- How well the computer-controlled alchemists play; games created before keep the basic AI.
ALTER TABLE games ADD COLUMN ai_level TEXT NOT NULL DEFAULT 'beginner';
