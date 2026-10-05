use std::collections::HashSet;

use rand::RngExt;
use rand::distr::Alphanumeric;
use sqlx::{SqliteConnection, SqlitePool};

use crate::db::{EventRecord, FanaticRecord, GameRecord, PlayerRecord};
use crate::domain::rules::{
    MAX_NAME_LENGTH, MAX_PLAYERS, MAX_TURNS, MIN_PLAYERS, STARTING_LABORATORIES, STARTING_MONEY,
};
use crate::domain::{Fanatic, FanaticPool, ItemCounts};
use crate::error::AppError;
use crate::service::write_transaction::begin_write;

const TOKEN_LENGTH: usize = 24;

/// A player created with a new game, with the token that makes up their private link.
#[derive(Debug, Clone)]
pub struct CreatedPlayer {
    /// Name of the player.
    pub name: String,
    /// Secret token of the player's link.
    pub token: String,
}

/// The result of [`create_game`].
#[derive(Debug, Clone)]
pub struct CreatedGame {
    /// Id of the new game.
    pub game_id: i64,
    /// Players in seat order.
    pub players: Vec<CreatedPlayer>,
}

/// Random choices made at setup, computed before touching the database.
struct Setup {
    first_seat: i64,
    hands: Vec<Vec<Fanatic>>,
    tokens: Vec<String>,
}

/// Sets up a new game: money, laboratories, fanatics and the item supply.
///
/// # Arguments
/// * `player_names` - names in seat order; surrounding spaces are trimmed.
/// * `max_turns` - number of turns before the game ends.
///
/// # Errors
/// Fails on an invalid player count, player name or number of turns, or on a database error.
pub async fn create_game(
    pool: &SqlitePool,
    player_names: &[String],
    max_turns: i64,
) -> Result<CreatedGame, AppError> {
    let names = validate_names(player_names)?;
    if !(1..=MAX_TURNS).contains(&max_turns) {
        return Err(AppError::InvalidTurnCount);
    }
    let setup = draw_setup(names.len())?;

    let mut tx = begin_write(pool).await?;
    let game_id = GameRecord::insert(
        &mut tx,
        max_turns,
        setup.first_seat,
        ItemCounts::initial_supply(),
    )
    .await?;
    let players = insert_players(&mut tx, game_id, &names, setup.hands, setup.tokens).await?;
    let first_name = players
        .get(setup.first_seat as usize)
        .map(|player| player.name.as_str())
        .unwrap_or_default();
    let message = format!("La partie commence. {first_name} joue en premier.");
    EventRecord::insert(&mut tx, game_id, 1, &message).await?;
    tx.commit().await?;

    Ok(CreatedGame { game_id, players })
}

fn validate_names(player_names: &[String]) -> Result<Vec<String>, AppError> {
    let names: Vec<String> = player_names
        .iter()
        .map(|name| name.trim().to_string())
        .collect();
    if !(MIN_PLAYERS..=MAX_PLAYERS).contains(&names.len()) {
        return Err(AppError::InvalidPlayerCount);
    }
    let valid_length = |name: &String| (1..=MAX_NAME_LENGTH).contains(&name.chars().count());
    let distinct: HashSet<&String> = names.iter().collect();
    if !names.iter().all(valid_length) || distinct.len() != names.len() {
        return Err(AppError::InvalidPlayerName);
    }
    Ok(names)
}

fn draw_setup(player_count: usize) -> Result<Setup, AppError> {
    let mut rng = rand::rng();
    let hands = FanaticPool::full()
        .deal(player_count, &mut rng)
        .ok_or(AppError::InvalidPlayerCount)?;
    let tokens = (0..player_count)
        .map(|_| {
            (&mut rng)
                .sample_iter(Alphanumeric)
                .take(TOKEN_LENGTH)
                .map(char::from)
                .collect()
        })
        .collect();
    let first_seat = rng.random_range(0..player_count as i64);
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
    tokens: Vec<String>,
) -> Result<Vec<CreatedPlayer>, AppError> {
    let mut players = Vec::with_capacity(names.len());
    for (seat, ((name, hand), token)) in names.iter().zip(hands).zip(tokens).enumerate() {
        let player_id = PlayerRecord::insert(
            conn,
            game_id,
            seat as i64,
            name,
            &token,
            STARTING_MONEY,
            STARTING_LABORATORIES,
        )
        .await?;
        for fanatic in hand {
            FanaticRecord::insert(conn, game_id, player_id, fanatic).await?;
        }
        players.push(CreatedPlayer {
            name: name.clone(),
            token,
        });
    }
    Ok(players)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| name.to_string()).collect()
    }

    #[test]
    fn rejects_bad_player_counts() {
        assert!(matches!(
            validate_names(&names(&["Ana"])),
            Err(AppError::InvalidPlayerCount)
        ));
        assert!(matches!(
            validate_names(&names(&["a", "b", "c", "d", "e", "f", "g"])),
            Err(AppError::InvalidPlayerCount)
        ));
    }

    #[test]
    fn rejects_blank_or_duplicate_names() {
        assert!(matches!(
            validate_names(&names(&["Ana", "  "])),
            Err(AppError::InvalidPlayerName)
        ));
        assert!(matches!(
            validate_names(&names(&["Ana", " Ana "])),
            Err(AppError::InvalidPlayerName)
        ));
    }

    #[test]
    fn trims_names() {
        assert_eq!(
            validate_names(&names(&[" Ana", "Bob "])).unwrap(),
            names(&["Ana", "Bob"])
        );
    }
}
