CREATE TABLE cult_markers (
    game_id INTEGER NOT NULL REFERENCES games (id),
    site INTEGER NOT NULL,
    cult TEXT NOT NULL,
    face_up INTEGER NOT NULL,
    PRIMARY KEY (game_id, site)
) STRICT;

CREATE TABLE rebels (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL REFERENCES games (id),
    site INTEGER NOT NULL
) STRICT;

CREATE INDEX rebels_game ON rebels (game_id);
