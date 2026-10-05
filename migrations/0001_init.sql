CREATE TABLE games (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    phase TEXT NOT NULL,
    turn INTEGER NOT NULL,
    max_turns INTEGER NOT NULL,
    first_seat INTEGER NOT NULL,
    active_seat INTEGER,
    supply_toratsa INTEGER NOT NULL,
    supply_xephis INTEGER NOT NULL,
    supply_yaboul INTEGER NOT NULL,
    collective_expense INTEGER NOT NULL DEFAULT 0,
    version INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE TABLE players (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL REFERENCES games (id),
    seat INTEGER NOT NULL,
    name TEXT NOT NULL,
    token TEXT NOT NULL UNIQUE,
    money INTEGER NOT NULL,
    laboratories INTEGER NOT NULL,
    toratsa_in_production INTEGER NOT NULL DEFAULT 0,
    xephis_in_production INTEGER NOT NULL DEFAULT 0,
    yaboul_in_production INTEGER NOT NULL DEFAULT 0,
    toratsa_in_stock INTEGER NOT NULL DEFAULT 0,
    xephis_in_stock INTEGER NOT NULL DEFAULT 0,
    yaboul_in_stock INTEGER NOT NULL DEFAULT 0,
    turn_expense INTEGER NOT NULL DEFAULT 0,
    announcement_done INTEGER NOT NULL DEFAULT 0,
    expense_done INTEGER NOT NULL DEFAULT 0,
    UNIQUE (game_id, seat)
);

CREATE TABLE fanatics (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL REFERENCES games (id),
    player_id INTEGER NOT NULL REFERENCES players (id),
    cult TEXT NOT NULL,
    strength INTEGER NOT NULL,
    in_hall INTEGER NOT NULL DEFAULT 0
);

CREATE TABLE event_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL REFERENCES games (id),
    turn INTEGER NOT NULL,
    message TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX players_game ON players (game_id);
CREATE INDEX fanatics_game ON fanatics (game_id);
CREATE INDEX event_log_game ON event_log (game_id);
