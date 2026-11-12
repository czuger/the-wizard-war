-- Level of one computer-controlled alchemist, for games pitting levels against each other;
-- NULL plays at the game's level.
ALTER TABLE players ADD COLUMN ai_level TEXT;
