use sqlx::SqliteConnection;

use crate::db::{GameRecord, PlayerRecord};
use crate::domain::Phase;
use crate::error::AppError;

/// The game and the acting player, loaded at the start of every player action.
pub struct PlayerAction {
    /// The game being played.
    pub game: GameRecord,
    /// The player acting, identified by their link token.
    pub player: PlayerRecord,
}

impl PlayerAction {
    /// Loads the game and the player, checking that the game is in `expected` phase.
    ///
    /// # Errors
    /// [`AppError::GameNotFound`], [`AppError::PlayerNotFound`] or [`AppError::WrongPhase`].
    pub async fn load(
        conn: &mut SqliteConnection,
        game_id: i64,
        token: &str,
        expected: Phase,
    ) -> Result<PlayerAction, AppError> {
        let game = GameRecord::find(conn, game_id).await?;
        let player = PlayerRecord::find_by_token(conn, game_id, token).await?;
        if game.phase != expected {
            return Err(AppError::WrongPhase);
        }
        Ok(PlayerAction { game, player })
    }
}
