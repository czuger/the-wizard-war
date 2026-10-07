-- Games may have 2 to 6 players; earlier games all had 6.
ALTER TABLE games ADD COLUMN player_count INTEGER NOT NULL DEFAULT 6;

-- Fanatic pawns left in the box when fewer than 6 players share the pool: nobody sees them,
-- but every player knows they may be among the unseen ones.
CREATE TABLE spare_fanatics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL REFERENCES games (id),
    cult TEXT NOT NULL,
    strength INTEGER NOT NULL
) STRICT;

CREATE INDEX spare_fanatics_game ON spare_fanatics (game_id);
