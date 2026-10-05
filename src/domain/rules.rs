//! Fixed numbers from the rules.

/// Money each player starts with, in écus.
pub const STARTING_MONEY: i64 = 5_000;

/// Laboratories each player starts with.
pub const STARTING_LABORATORIES: i64 = 1;

/// Number of players in every game: one human and the computer-controlled alchemists.
pub const PLAYER_COUNT: usize = 6;

/// Seat of the human player.
pub const HUMAN_SEAT: i64 = 0;

/// Names of the computer-controlled alchemists, seated after the human player.
pub const AI_NAMES: [&str; PLAYER_COUNT - 1] =
    ["Morgane", "Théophraste", "Ysolde", "Barnabé", "Mélusine"];

/// Smallest banknote: expenses are paid in multiples of this amount.
pub const EXPENSE_STEP: i64 = 100;

/// Number of turns suggested for a first game.
pub const DEFAULT_TURNS: i64 = 8;

/// Most turns a game can be set to last.
pub const MAX_TURNS: i64 = 30;

/// Longest accepted player name, in characters.
pub const MAX_NAME_LENGTH: usize = 30;

/// Cult markers of each cult in the box.
pub const MARKERS_PER_CULT: i64 = 21;

/// Rebels pawns in the box.
pub const REBEL_COUNT: i64 = 7;
