//! Fixed numbers from the rules.

/// Money each player starts with, in écus.
pub const STARTING_MONEY: i64 = 5_000;

/// Laboratories each player starts with.
pub const STARTING_LABORATORIES: i64 = 1;

/// Fewest players in a game.
pub const MIN_PLAYERS: usize = 2;

/// Most players in a game.
pub const MAX_PLAYERS: usize = 6;

/// Smallest banknote: expenses are paid in multiples of this amount.
pub const EXPENSE_STEP: i64 = 100;

/// Number of turns suggested for a first game.
pub const DEFAULT_TURNS: i64 = 8;

/// Most turns a game can be set to last.
pub const MAX_TURNS: i64 = 30;

/// Longest accepted player name, in characters.
pub const MAX_NAME_LENGTH: usize = 30;
