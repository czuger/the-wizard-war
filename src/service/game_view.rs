use sqlx::SqlitePool;

use crate::db::{EventRecord, FanaticRecord, GameRecord, PlayerRecord};
use crate::domain::{ItemCounts, Phase};
use crate::error::AppError;
use crate::service::produce_items::required_items;

const EVENT_LIMIT: i64 = 40;

/// What everyone can see about one player.
#[derive(Debug, Clone)]
pub struct PlayerSummary {
    /// Name of the player.
    pub name: String,
    /// Écus owned.
    pub money: i64,
    /// Laboratories owned.
    pub laboratories: i64,
    /// Number of items in stock (they lie face down, so only the count is public).
    pub stock_total: i64,
    /// French description of what the player is doing in the current phase.
    pub status: &'static str,
    /// Whether this is the player viewing the page.
    pub is_me: bool,
    /// Whether the computer plays this alchemist.
    pub is_ai: bool,
    /// Whether this player plays first this turn.
    pub is_first: bool,
}

/// The game as one player is allowed to see it.
#[derive(Debug, Clone)]
pub struct GameView {
    /// Id of the game.
    pub game_id: i64,
    /// Current turn, starting at 1.
    pub turn: i64,
    /// Turn after which the game ends.
    pub max_turns: i64,
    /// Current phase.
    pub phase: Phase,
    /// Seat expected to act in the production phase.
    pub active_seat: Option<i64>,
    /// Magic items left in the supply.
    pub supply: ItemCounts,
    /// The viewing player.
    pub me: PlayerRecord,
    /// The viewing player's fanatics, including the one in the hall.
    pub my_fanatics: Vec<FanaticRecord>,
    /// Every player, in seat order.
    pub players: Vec<PlayerSummary>,
    /// Number of face-down fanatics in the great hall.
    pub hall_count: usize,
    /// Latest log messages, newest first.
    pub events: Vec<EventRecord>,
    /// Change counter, used by the page to know when to refresh.
    pub version: i64,
}

impl GameView {
    /// Loads the game as seen by the player owning `token`.
    ///
    /// # Errors
    /// [`AppError::GameNotFound`], [`AppError::PlayerNotFound`] or a database error.
    pub async fn load(pool: &SqlitePool, game_id: i64, token: &str) -> Result<GameView, AppError> {
        let mut conn = pool.acquire().await?;
        let game = GameRecord::find(&mut conn, game_id).await?;
        let me = PlayerRecord::find_by_token(&mut conn, game_id, token).await?;
        let players = PlayerRecord::list_for_game(&mut conn, game_id).await?;
        let my_fanatics = FanaticRecord::list_for_player(&mut conn, me.id).await?;
        let hall_count = FanaticRecord::list_in_hall(&mut conn, game_id).await?.len();
        let events = EventRecord::latest(&mut conn, game_id, EVENT_LIMIT).await?;
        let summaries = players
            .iter()
            .map(|player| summarize(&game, player, me.id))
            .collect();
        Ok(GameView {
            game_id,
            turn: game.turn,
            max_turns: game.max_turns,
            phase: game.phase,
            active_seat: game.active_seat,
            supply: game.supply,
            me,
            my_fanatics,
            players: summaries,
            hall_count,
            events,
            version: game.version,
        })
    }

    /// Whether the viewing player still has to make this turn's announcement.
    pub fn can_announce(&self) -> bool {
        self.phase == Phase::Announcements && !self.me.announcement_done
    }

    /// Whether the viewing player still has to pay this turn's expense.
    pub fn can_pay(&self) -> bool {
        self.phase == Phase::Expenses && !self.me.expense_done
    }

    /// Whether the viewing player is the one making items now.
    pub fn can_produce(&self) -> bool {
        self.phase == Phase::Production && self.active_seat == Some(self.me.seat)
    }

    /// Number of items the viewing player must take when producing.
    pub fn required_items(&self) -> i64 {
        required_items(self.me.laboratories, &self.supply)
    }

    /// Whether the game is over.
    pub fn is_finished(&self) -> bool {
        self.phase == Phase::Finished
    }

    /// Fanatics the viewing player may still send to the hall.
    pub fn available_fanatics(&self) -> Vec<&FanaticRecord> {
        self.my_fanatics
            .iter()
            .filter(|record| !record.in_hall)
            .collect()
    }

    /// The viewing player's fanatic waiting in the hall, if any.
    pub fn my_fanatic_in_hall(&self) -> Option<&FanaticRecord> {
        self.my_fanatics.iter().find(|record| record.in_hall)
    }
}

fn summarize(game: &GameRecord, player: &PlayerRecord, my_id: i64) -> PlayerSummary {
    PlayerSummary {
        name: player.name.clone(),
        money: player.money,
        laboratories: player.laboratories,
        stock_total: player.stock.total(),
        status: status_of(game, player),
        is_me: player.id == my_id,
        is_ai: player.is_ai,
        is_first: player.seat == game.first_seat,
    }
}

fn status_of(game: &GameRecord, player: &PlayerRecord) -> &'static str {
    match game.phase {
        Phase::Announcements if player.announcement_done => "Annonce faite",
        Phase::Announcements => "Choisit un fanatique…",
        Phase::Expenses if player.expense_done => "Dépense faite",
        Phase::Expenses => "Prépare sa dépense…",
        Phase::Production if game.active_seat == Some(player.seat) => "Fabrique ses objets…",
        Phase::Production => "Attend",
        Phase::Finished => "",
    }
}
