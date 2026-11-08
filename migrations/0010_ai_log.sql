-- Detailed log of every AI decision and of each turn's hidden state, kept to evaluate the
-- AI. Never shown to players: it reveals hands, stocks and loads.
CREATE TABLE ai_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    game_id INTEGER NOT NULL REFERENCES games (id),
    turn INTEGER NOT NULL,
    phase TEXT NOT NULL,
    -- NULL for entries about the whole table, such as end-of-turn summaries.
    player_id INTEGER REFERENCES players (id),
    body TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
) STRICT;

CREATE INDEX ai_log_game ON ai_log (game_id);
