use rand::RngExt;
use rand::distr::Alphanumeric;
use sqlx::{SqliteConnection, SqlitePool};

use crate::db::{EventRecord, FanaticRecord, GameRecord, PlayerRecord};
use crate::domain::rules::{AI_NAMES, HUMAN_SEAT, MAX_NAME_LENGTH, MAX_TURNS, PLAYER_COUNT};
use crate::domain::{Fanatic, FanaticPool, ItemCounts};
use crate::error::AppError;
use crate::service::player_action::PlayerAction;
use crate::service::write_transaction::begin_write;

const TOKEN_LENGTH: usize = 24;

/// The result of [`create_game`].
#[derive(Debug, Clone)]
pub struct CreatedGame {
    /// Id of the new game.
    pub game_id: i64,
    /// Secret token of the human player's link.
    pub human_token: String,
}

/// Random choices made at setup, computed before touching the database.
struct Setup {
    first_seat: i64,
    hands: Vec<Vec<Fanatic>>,
    tokens: Vec<String>,
}

/// Sets up a new game for one human player against the computer-controlled alchemists:
/// money, laboratories, fanatics and the item supply. The AIs then make their first
/// announcements.
///
/// # Arguments
/// * `human_name` - name of the human player, seated first; surrounding spaces are trimmed.
/// * `max_turns` - number of turns before the game ends.
///
/// # Errors
/// Fails on an invalid name or number of turns, or on a database error.
pub async fn create_game(
    pool: &SqlitePool,
    human_name: &str,
    max_turns: i64,
) -> Result<CreatedGame, AppError> {
    let names = seat_names(human_name)?;
    if !(1..=MAX_TURNS).contains(&max_turns) {
        return Err(AppError::InvalidTurnCount);
    }
    let setup = draw_setup()?;
    let human_token = setup
        .tokens
        .get(HUMAN_SEAT as usize)
        .cloned()
        .ok_or(AppError::InvalidPlayerCount)?;

    let mut tx = begin_write(pool).await?;
    let game_id = GameRecord::insert(
        &mut tx,
        max_turns,
        setup.first_seat,
        ItemCounts::initial_supply(),
    )
    .await?;
    insert_players(&mut tx, game_id, &names, setup.hands, &setup.tokens).await?;
    let first_name = names
        .get(setup.first_seat as usize)
        .map(String::as_str)
        .unwrap_or_default();
    let message = format!("La partie commence. {first_name} joue en premier.");
    EventRecord::insert(&mut tx, game_id, 1, &message).await?;
    let game = GameRecord::find(&mut tx, game_id).await?;
    PlayerAction::finish(tx, game).await?;
    Ok(CreatedGame {
        game_id,
        human_token,
    })
}

/// Returns the names in seat order: the human player, then the AIs.
fn seat_names(human_name: &str) -> Result<Vec<String>, AppError> {
    let name = human_name.trim();
    let valid_length = (1..=MAX_NAME_LENGTH).contains(&name.chars().count());
    if !valid_length || AI_NAMES.contains(&name) {
        return Err(AppError::InvalidPlayerName);
    }
    let mut names = vec![name.to_string()];
    names.extend(AI_NAMES.iter().map(|ai_name| ai_name.to_string()));
    Ok(names)
}

fn draw_setup() -> Result<Setup, AppError> {
    let mut rng = rand::rng();
    let hands = FanaticPool::full()
        .deal(PLAYER_COUNT, &mut rng)
        .ok_or(AppError::InvalidPlayerCount)?;
    let tokens = (0..PLAYER_COUNT)
        .map(|_| {
            (&mut rng)
                .sample_iter(Alphanumeric)
                .take(TOKEN_LENGTH)
                .map(char::from)
                .collect()
        })
        .collect();
    let first_seat = rng.random_range(0..PLAYER_COUNT as i64);
    Ok(Setup {
        first_seat,
        hands,
        tokens,
    })
}

async fn insert_players(
    conn: &mut SqliteConnection,
    game_id: i64,
    names: &[String],
    hands: Vec<Vec<Fanatic>>,
    tokens: &[String],
) -> Result<(), AppError> {
    for (seat, ((name, hand), token)) in names.iter().zip(hands).zip(tokens).enumerate() {
        let seat = seat as i64;
        let player_id =
            PlayerRecord::insert(conn, game_id, seat, name, token, seat != HUMAN_SEAT).await?;
        for fanatic in hand {
            FanaticRecord::insert(conn, game_id, player_id, fanatic).await?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_sits_first_followed_by_the_ais() {
        let names = seat_names(" Ana ").unwrap();
        assert_eq!(names.len(), PLAYER_COUNT);
        assert_eq!(names[HUMAN_SEAT as usize], "Ana");
        assert_eq!(names[1..], AI_NAMES.map(String::from));
    }

    #[test]
    fn rejects_blank_long_or_ai_names() {
        for name in ["  ", &"a".repeat(MAX_NAME_LENGTH + 1), AI_NAMES[0]] {
            assert!(matches!(seat_names(name), Err(AppError::InvalidPlayerName)));
        }
    }
}
