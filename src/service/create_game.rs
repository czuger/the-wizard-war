use rand::RngExt;
use rand::distr::Alphanumeric;
use rand::seq::SliceRandom;
use sqlx::{SqliteConnection, SqlitePool};

use crate::db::{
    EventRecord, FanaticRecord, GameRecord, NewPlayer, PlayerRecord, SpareFanaticRecord,
};
use crate::domain::rules::{
    AI_NAMES, HUMAN_SEAT, MAX_NAME_LENGTH, MAX_PLAYER_COUNT, MAX_TURNS, MIN_PLAYER_COUNT,
};
use crate::domain::{
    AiLevel, Archetype, District, Fanatic, FanaticDeal, FanaticPool, ItemCounts, LogSection,
};
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

/// What the human player chooses on the creation form.
#[derive(Debug, Clone)]
pub struct GameSettings<'a> {
    /// Name of the human player, seated first; surrounding spaces are trimmed.
    pub human_name: &'a str,
    /// Number of turns before the game ends.
    pub max_turns: i64,
    /// District of the human player's laboratories; the AIs get others.
    pub district: District,
    /// How well every computer-controlled alchemist plays.
    pub ai_level: AiLevel,
    /// Number of players, the human included, from 2 to 6.
    pub player_count: usize,
}

/// Random choices made at setup, computed before touching the database.
struct Setup {
    first_seat: i64,
    fanatics: FanaticDeal,
    tokens: Vec<String>,
    temperaments: Vec<(Archetype, i64)>,
}

/// Sets up a new game for one human player against the computer-controlled alchemists:
/// money, laboratories, fanatics and the item supply. The AIs then make their first
/// announcements.
///
/// # Errors
/// Fails on an invalid name, number of turns or number of players, or on a database error.
pub async fn create_game(
    pool: &SqlitePool,
    settings: &GameSettings<'_>,
) -> Result<CreatedGame, AppError> {
    let GameSettings {
        max_turns,
        ai_level,
        player_count,
        ..
    } = *settings;
    if !(MIN_PLAYER_COUNT..=MAX_PLAYER_COUNT).contains(&player_count) {
        return Err(AppError::InvalidPlayerCount);
    }
    let names = seat_names(settings.human_name, player_count)?;
    let districts = seat_districts(settings.district, player_count);
    if !(1..=MAX_TURNS).contains(&max_turns) {
        return Err(AppError::InvalidTurnCount);
    }
    let setup = draw_setup(player_count)?;
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
        ai_level,
        player_count as i64,
    )
    .await?;
    let seats = SeatSetup {
        names: &names,
        districts: &districts,
        tokens: &setup.tokens,
        temperaments: &setup.temperaments,
    };
    insert_players(&mut tx, game_id, &seats, setup.fanatics.hands).await?;
    for fanatic in setup.fanatics.spare {
        SpareFanaticRecord::insert(&mut tx, game_id, fanatic).await?;
    }
    let first_name = names
        .get(setup.first_seat as usize)
        .map(String::as_str)
        .unwrap_or_default();
    let message = format!(
        "La partie commence (alchimistes de niveau {}). {first_name} joue en premier.",
        ai_level.label()
    );
    EventRecord::insert(&mut tx, game_id, 1, LogSection::Turn, &message).await?;
    let game = GameRecord::find(&mut tx, game_id).await?;
    PlayerAction::finish(tx, game).await?;
    Ok(CreatedGame {
        game_id,
        human_token,
    })
}

/// Returns the names in seat order: the human player, then the AIs.
fn seat_names(human_name: &str, player_count: usize) -> Result<Vec<String>, AppError> {
    let name = human_name.trim();
    let valid_length = (1..=MAX_NAME_LENGTH).contains(&name.chars().count());
    if !valid_length || AI_NAMES.contains(&name) {
        return Err(AppError::InvalidPlayerName);
    }
    let mut names = vec![name.to_string()];
    names.extend(
        AI_NAMES
            .iter()
            .take(player_count - 1)
            .map(|ai_name| ai_name.to_string()),
    );
    Ok(names)
}

/// Returns the districts in seat order: the human player's, then others for the AIs.
fn seat_districts(human: District, player_count: usize) -> Vec<District> {
    let mut districts = vec![human];
    districts.extend(
        District::ALL
            .into_iter()
            .filter(|&district| district != human)
            .take(player_count - 1),
    );
    districts
}

fn draw_setup(player_count: usize) -> Result<Setup, AppError> {
    let mut rng = rand::rng();
    let fanatics = FanaticPool::full()
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
    let mut archetypes = Archetype::DRAWN;
    archetypes.shuffle(&mut rng);
    let mut temperaments = vec![(Archetype::Balanced, 0)];
    for &archetype in archetypes.iter().take(player_count - 1) {
        temperaments.push((archetype, rng.random::<i64>()));
    }
    Ok(Setup {
        first_seat,
        fanatics,
        tokens,
        temperaments,
    })
}

/// What each seat is given, in seat order.
struct SeatSetup<'a> {
    names: &'a [String],
    districts: &'a [District],
    tokens: &'a [String],
    /// Hidden playing style and its seed; the human player's is balanced.
    temperaments: &'a [(Archetype, i64)],
}

async fn insert_players(
    conn: &mut SqliteConnection,
    game_id: i64,
    seats: &SeatSetup<'_>,
    hands: Vec<Vec<Fanatic>>,
) -> Result<(), AppError> {
    let seat_data = seats
        .names
        .iter()
        .zip(seats.districts)
        .zip(seats.tokens)
        .zip(seats.temperaments);
    for (seat, ((((name, district), token), temperament), hand)) in seat_data.zip(hands).enumerate()
    {
        let seat = seat as i64;
        let player = NewPlayer {
            game_id,
            seat,
            name,
            token,
            is_ai: seat != HUMAN_SEAT,
            district: *district,
            archetype: temperament.0,
            temperament_seed: temperament.1,
        };
        let player_id = PlayerRecord::insert(conn, player).await?;
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
        let names = seat_names(" Ana ", MAX_PLAYER_COUNT).unwrap();
        assert_eq!(names.len(), MAX_PLAYER_COUNT);
        assert_eq!(names[HUMAN_SEAT as usize], "Ana");
        assert_eq!(names[1..], AI_NAMES.map(String::from));
        assert_eq!(
            seat_names("Ana", 3).unwrap(),
            ["Ana", AI_NAMES[0], AI_NAMES[1]]
        );
    }

    #[test]
    fn every_seat_gets_its_own_district() {
        let districts = seat_districts(District::Green, MAX_PLAYER_COUNT);
        assert_eq!(districts.len(), MAX_PLAYER_COUNT);
        assert_eq!(districts[0], District::Green);
        assert!(
            District::ALL
                .iter()
                .all(|district| districts.contains(district))
        );
        let districts = seat_districts(District::Green, 2);
        assert_eq!(districts.len(), 2);
        assert_ne!(districts[1], District::Green);
    }

    #[test]
    fn rejects_blank_long_or_ai_names() {
        for name in ["  ", &"a".repeat(MAX_NAME_LENGTH + 1), AI_NAMES[0]] {
            assert!(matches!(
                seat_names(name, MAX_PLAYER_COUNT),
                Err(AppError::InvalidPlayerName)
            ));
        }
    }
}
