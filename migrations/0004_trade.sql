ALTER TABLE players ADD COLUMN district TEXT NOT NULL DEFAULT 'blue';
UPDATE players SET district = CASE seat
    WHEN 1 THEN 'white'
    WHEN 2 THEN 'yellow'
    WHEN 3 THEN 'pink'
    WHEN 4 THEN 'black'
    WHEN 5 THEN 'green'
    ELSE 'blue'
END;
ALTER TABLE players ADD COLUMN moves_done INTEGER NOT NULL DEFAULT 0;

ALTER TABLE games ADD COLUMN city_sales_left INTEGER NOT NULL DEFAULT 0;
ALTER TABLE games ADD COLUMN city_passes INTEGER NOT NULL DEFAULT 0;

ALTER TABLE rebels ADD COLUMN must_fight INTEGER NOT NULL DEFAULT 0;

CREATE TABLE pawns (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL REFERENCES games (id),
    player_id INTEGER NOT NULL REFERENCES players (id),
    kind TEXT NOT NULL,
    site INTEGER,
    item TEXT,
    stopped_until_turn INTEGER NOT NULL DEFAULT 0
) STRICT;

CREATE TABLE sales (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL REFERENCES games (id),
    turn INTEGER NOT NULL,
    site INTEGER NOT NULL
) STRICT;

CREATE INDEX pawns_game ON pawns (game_id);
CREATE INDEX sales_game_turn ON sales (game_id, turn);
