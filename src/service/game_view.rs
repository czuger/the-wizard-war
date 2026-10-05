use sqlx::SqlitePool;

use crate::db::{EventRecord, FanaticRecord, GameRecord, PlayerRecord, RebelRecord};
use crate::domain::{ItemCounts, Phase, Wealth};
use crate::error::AppError;
use crate::service::board_loader::load_board_state;
use crate::service::board_view::BoardView;
use crate::service::city_quota_row::CityQuotaRow;
use crate::service::city_view::CityView;
use crate::service::produce_items::required_items;
use crate::service::seat_cycle::seats_in_turn_order;
use crate::service::trade_view::TradeView;

const EVENT_LIMIT: i64 = 120;

/// What everyone can see about one player.
#[derive(Debug, Clone)]
pub struct PlayerSummary {
    /// Name of the player.
    pub name: String,
    /// Écus owned, known only for the viewing player.
    pub money: Option<i64>,
    /// Rough wealth, which is all the other players can tell from the banknote pile.
    pub wealth: Wealth,
    /// Laboratories owned.
    pub laboratories: i64,
    /// Number of items in stock (they lie face down, so only the count is public).
    pub stock_total: i64,
    /// French name of the district holding the player's laboratories.
    pub district: &'static str,
    /// Key of the district, which is also the colour of the player's pawns.
    pub district_key: &'static str,
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
    /// Every player, in seat order from the turn's first player.
    pub players: Vec<PlayerSummary>,
    /// Number of fanatics in the great hall.
    pub hall_count: usize,
    /// Laboratories and the great hall, drawn on the map.
    pub city: CityView,
    /// The map, with the viewing player's phase II choices.
    pub board: BoardView,
    /// Peddlers, caravans, fights and purchases, with the viewing player's choices.
    pub trade: TradeView,
    /// Items the city may still buy this turn (phase VI).
    pub city_sales_left: i64,
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
        let hall = FanaticRecord::list_in_hall(&mut conn, game_id).await?;
        let events = EventRecord::latest(&mut conn, game_id, EVENT_LIMIT).await?;
        let board_state = load_board_state(&mut conn, game_id).await?;
        let rebels = RebelRecord::list_for_game(&mut conn, game_id).await?;
        let trade = TradeView::load(&mut conn, &game, &me, &players, &board_state).await?;
        let my_turn = game.active_seat == Some(me.seat);
        let board = BoardView::build(
            &board_state,
            &rebels,
            (my_turn && game.phase == Phase::Conversions).then_some(game.turn),
            my_turn && game.phase == Phase::RebelMovement,
            &trade.tokens,
        );
        let city = CityView::build(&players, &hall, &trade.tokens, me.id);
        let summaries = seats_in_turn_order(&game)
            .into_iter()
            .filter_map(|seat| players.iter().find(|player| player.seat == seat))
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
            hall_count: hall.len(),
            city,
            board,
            trade,
            city_sales_left: game.city_sales_left,
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

    /// Laboratories on the board, all players together.
    pub fn total_laboratories(&self) -> i64 {
        self.players.iter().map(|player| player.laboratories).sum()
    }

    /// Table 2, with the row matching the laboratories now on the board flagged.
    pub fn city_quota_rows(&self) -> Vec<CityQuotaRow> {
        CityQuotaRow::table(self.total_laboratories())
    }

    /// Whether the viewing player is the one converting a site now.
    pub fn can_convert(&self) -> bool {
        self.phase == Phase::Conversions && self.active_seat == Some(self.me.seat)
    }

    /// Whether the viewing player has to move the rebels now.
    pub fn can_move_rebels(&self) -> bool {
        self.phase == Phase::RebelMovement && self.active_seat == Some(self.me.seat)
    }

    /// Whether the viewing player must choose the pawn the rebels attack.
    pub fn can_attack(&self) -> bool {
        self.trade.fight.is_some()
    }

    /// Whether the viewing player must move their peddlers and caravans now.
    pub fn can_move_pawns(&self) -> bool {
        self.my_sequential_turn(Phase::Peddlers) && !self.me.moves_done
    }

    /// Whether the viewing player may sell with their peddlers and caravans now.
    pub fn can_sell_items(&self) -> bool {
        self.my_sequential_turn(Phase::Peddlers) && self.me.moves_done
    }

    /// Whether the viewing player may sell an item in the city now.
    pub fn can_sell_in_city(&self) -> bool {
        self.my_sequential_turn(Phase::CitySales)
    }

    /// Whether the viewing player may buy from the bank now.
    pub fn can_buy(&self) -> bool {
        self.trade.purchase.is_some()
    }

    fn my_sequential_turn(&self, phase: Phase) -> bool {
        self.phase == phase && self.active_seat == Some(self.me.seat)
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
        money: (player.id == my_id).then_some(player.money),
        wealth: Wealth::of(player.money),
        laboratories: player.laboratories,
        stock_total: player.stock.total(),
        district: player.district.label(),
        district_key: player.district.key(),
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
        Phase::Conversions if game.active_seat == Some(player.seat) => "Convertit un lieu…",
        Phase::RebelMovement if game.active_seat == Some(player.seat) => "Déplace les rebelles…",
        Phase::Conversions | Phase::RebelMovement => "Attend",
        Phase::RebelFights if game.active_seat == Some(player.seat) => "Lance les rebelles…",
        Phase::Production if game.active_seat == Some(player.seat) => "Fabrique ses objets…",
        Phase::Peddlers if game.active_seat == Some(player.seat) => "Déplace ses pions…",
        Phase::CitySales if game.active_seat == Some(player.seat) => "Vend en ville…",
        Phase::Purchases if game.active_seat == Some(player.seat) => "Fait ses achats…",
        Phase::RebelFights
        | Phase::Production
        | Phase::Peddlers
        | Phase::CitySales
        | Phase::Purchases => "Attend",
        Phase::Finished => "",
    }
}
