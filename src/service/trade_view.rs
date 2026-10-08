use sqlx::SqliteConnection;

use crate::db::{GameRecord, PawnRecord, PlayerRecord, SaleRecord};
use crate::domain::board::site_by_id;
use crate::domain::market::{buys_without_limit, city_price};
use crate::domain::rules::LABORATORY_PRICE;
use crate::domain::travel::destinations;
use crate::domain::{BoardState, Holdings, Location, MagicItem, PawnKind, Phase, SiteId};
use crate::error::AppError;
use crate::service::pawn_rules::{can_transfer, is_movable, sale_offer};
use crate::service::purchases::holdings_of;
use crate::service::rebel_fights::pending_fight;

/// A peddler or caravan drawn on the map, on its site or in its owner's district; its item
/// stays hidden.
#[derive(Debug, Clone)]
pub struct PawnToken {
    /// Id of the pawn, used to find the viewing player's choices for it.
    pub pawn_id: i64,
    /// Id of the pawn's owner.
    pub owner_id: i64,
    /// Where the pawn stands.
    pub location: Location,
    /// Path of the pawn's image, in its owner's colour.
    pub image: String,
    /// French description shown on hover, e.g. « Caravane de Morgane ».
    pub title: String,
}

impl PawnToken {
    /// Form value of the pawn's location, where the map script starts its move arrow.
    pub fn from_value(&self) -> String {
        self.location.form_value()
    }
}

/// One of the viewing player's pawns.
#[derive(Debug, Clone)]
pub struct PawnView {
    /// Path of the pawn's image.
    pub image: String,
    /// French description: kind, place and load.
    pub label: String,
}

/// A stop a pawn may end its move on.
#[derive(Debug, Clone)]
pub struct StopOption {
    /// Form value, decoded by [`Location::from_form_value`].
    pub value: String,
    /// French name of the stop.
    pub label: String,
}

/// A pawn the viewing player may move in phase V.
#[derive(Debug, Clone)]
pub struct MoveChoice {
    /// Id of the pawn, used in the form field names.
    pub pawn_id: i64,
    /// French description of the pawn.
    pub label: String,
    /// Whether the pawn is in the city and must load an item to leave.
    pub in_city: bool,
    /// Where it may go.
    pub destinations: Vec<StopOption>,
}

impl MoveChoice {
    /// Form values of the destinations, separated by spaces, for the map script.
    pub fn targets(&self) -> String {
        self.destinations
            .iter()
            .map(|stop| stop.value.as_str())
            .collect::<Vec<_>>()
            .join(" ")
    }
}

/// An item of the viewing player's stock.
#[derive(Debug, Clone)]
pub struct CargoOption {
    /// Form value, from [`MagicItem::key`].
    pub key: &'static str,
    /// Path of the item's token image.
    pub image: &'static str,
    /// French name.
    pub label: &'static str,
    /// Number of items in stock.
    pub count: i64,
    /// Price the city pays for it in phase VI.
    pub city_price: i64,
}

/// A pawn of the viewing player that may sell its item where it stands.
#[derive(Debug, Clone)]
pub struct SaleChoice {
    /// Id of the pawn.
    pub pawn_id: i64,
    /// French description of the sale.
    pub label: String,
    /// Whether the form ticks it at first: only one pawn per site that buys a single item.
    pub checked: bool,
}

/// An item the viewing player may pass from one of their pawns to another on the same site.
#[derive(Debug, Clone)]
pub struct TransferChoice {
    /// Id of the pawn that gives its item.
    pub from_pawn_id: i64,
    /// Id of the pawn that receives it.
    pub to_pawn_id: i64,
    /// French description, e.g. « Passer un grimoire Xéphis à la caravane vide ».
    pub label: String,
}

impl TransferChoice {
    /// Form value of the transfer: both pawn ids, giver first.
    pub fn value(&self) -> String {
        format!("{}-{}", self.from_pawn_id, self.to_pawn_id)
    }
}

/// A pawn the rebels may attack.
#[derive(Debug, Clone)]
pub struct FightTarget {
    /// Id of the pawn.
    pub pawn_id: i64,
    /// French description, e.g. « Colporteur de Ysolde ».
    pub label: String,
}

/// The fight the viewing player must settle as first player.
#[derive(Debug, Clone)]
pub struct FightView {
    /// Id of the village where the rebels arrived.
    pub site_id: u8,
    /// Name of the village where the rebels arrived.
    pub site: &'static str,
    /// The pawns there.
    pub targets: Vec<FightTarget>,
}

/// What the viewing player may buy in phase VII.
#[derive(Debug, Clone)]
pub struct PurchaseView {
    /// What the player owns.
    pub holdings: Holdings,
    /// Price of a peddler.
    pub peddler_price: i64,
    /// Price of a caravan.
    pub caravan_price: i64,
    /// Price of a laboratory.
    pub laboratory_price: i64,
}

/// Peddlers, caravans, fights and sales as the viewing player sees them.
#[derive(Debug, Clone, Default)]
pub struct TradeView {
    /// Every pawn, on the map or in the city, items hidden.
    pub tokens: Vec<PawnToken>,
    /// The viewing player's pawns.
    pub my_pawns: Vec<PawnView>,
    /// Pawns to move, when it is the viewer's turn to move in phase V.
    pub move_choices: Vec<MoveChoice>,
    /// The viewing player's stock items.
    pub cargo_options: Vec<CargoOption>,
    /// Pawns that may sell, when it is the viewer's turn to sell in phase V.
    pub sale_choices: Vec<SaleChoice>,
    /// Items to pass between pawns, when it is the viewer's turn to sell in phase V.
    pub transfer_choices: Vec<TransferChoice>,
    /// The fight to settle, when the viewer is the first player in a fight.
    pub fight: Option<FightView>,
    /// The purchase form, when it is the viewer's turn in phase VII.
    pub purchase: Option<PurchaseView>,
}

impl TradeView {
    /// Loads the trade part of the page for `me`.
    ///
    /// # Errors
    /// Fails on a database error.
    pub(crate) async fn load(
        conn: &mut SqliteConnection,
        game: &GameRecord,
        me: &PlayerRecord,
        players: &[PlayerRecord],
        board: &BoardState,
    ) -> Result<TradeView, AppError> {
        let pawns = PawnRecord::list_for_game(conn, game.id).await?;
        let my_turn = game.active_seat == Some(me.seat);
        let mine: Vec<PawnRecord> = pawns
            .iter()
            .filter(|pawn| pawn.player_id == me.id)
            .copied()
            .collect();
        let fight = match game.phase {
            Phase::RebelFights if my_turn => pending_fight_view(conn, game.id, players).await?,
            _ => None,
        };
        let sold = SaleRecord::sites_sold(conn, game.id, game.turn).await?;
        let selling = my_turn && game.phase == Phase::Peddlers && me.moves_done;
        Ok(TradeView {
            tokens: tokens(&pawns, players),
            my_pawns: mine
                .iter()
                .map(|pawn| pawn_view(pawn, me, game.turn))
                .collect(),
            move_choices: if my_turn && game.phase == Phase::Peddlers && !me.moves_done {
                move_choices(&mine, me, game.turn)
            } else {
                Vec::new()
            },
            cargo_options: cargo_options(me),
            sale_choices: if selling {
                sale_choices(&mine, board, &sold)
            } else {
                Vec::new()
            },
            transfer_choices: if selling {
                transfer_choices(&mine)
            } else {
                Vec::new()
            },
            fight,
            purchase: (my_turn && game.phase == Phase::Purchases).then(|| PurchaseView {
                holdings: holdings_of(game, me, players, &pawns),
                peddler_price: PawnKind::Peddler.price(),
                caravan_price: PawnKind::Caravan.price(),
                laboratory_price: LABORATORY_PRICE,
            }),
        })
    }
}

fn token_image(kind: PawnKind, owner: Option<&PlayerRecord>) -> String {
    let colour = owner.map_or("white", |owner| owner.district.key());
    format!("/img/token-{}-{colour}.png", kind.key())
}

fn capitalised(text: &str) -> String {
    let mut chars = text.chars();
    chars.next().map_or_else(String::new, |first| {
        first.to_uppercase().chain(chars).collect()
    })
}

fn location_name(location: Location) -> String {
    match location {
        Location::City => "en ville".to_string(),
        Location::Site(site) => format!("à {}", site_by_id(site).map_or("?", |site| site.name)),
    }
}

fn tokens(pawns: &[PawnRecord], players: &[PlayerRecord]) -> Vec<PawnToken> {
    pawns
        .iter()
        .map(|pawn| {
            let owner = players.iter().find(|player| player.id == pawn.player_id);
            PawnToken {
                pawn_id: pawn.id,
                owner_id: pawn.player_id,
                location: pawn.location,
                image: token_image(pawn.kind, owner),
                title: format!(
                    "{} de {}",
                    capitalised(pawn.kind.label()),
                    owner.map_or("?", |owner| owner.name.as_str())
                ),
            }
        })
        .collect()
}

fn describe_pawn(pawn: &PawnRecord, turn: i64) -> String {
    let load = match pawn.item {
        Some(item) => format!(", avec {}", item.with_article()),
        None => ", vide".to_string(),
    };
    let stopped = if pawn.can_move(turn) {
        ""
    } else {
        " (arrêté ce tour-ci)"
    };
    format!(
        "{} {}{load}{stopped}",
        capitalised(pawn.kind.label()),
        location_name(pawn.location)
    )
}

fn pawn_view(pawn: &PawnRecord, me: &PlayerRecord, turn: i64) -> PawnView {
    PawnView {
        image: token_image(pawn.kind, Some(me)),
        label: describe_pawn(pawn, turn),
    }
}

fn move_choices(mine: &[PawnRecord], me: &PlayerRecord, turn: i64) -> Vec<MoveChoice> {
    mine.iter()
        .filter(|pawn| is_movable(pawn, turn, &me.stock))
        .map(|pawn| MoveChoice {
            pawn_id: pawn.id,
            label: describe_pawn(pawn, turn),
            in_city: pawn.location == Location::City,
            destinations: destinations(pawn.kind, pawn.location)
                .into_iter()
                .map(|stop| StopOption {
                    value: stop.form_value(),
                    label: match stop {
                        Location::City => "Rentrer en ville".to_string(),
                        Location::Site(site) => site_by_id(site).map_or("?".to_string(), |site| {
                            format!("{}. {}", site.id.0, site.describe())
                        }),
                    },
                })
                .collect(),
        })
        .collect()
}

fn cargo_options(me: &PlayerRecord) -> Vec<CargoOption> {
    MagicItem::ALL
        .into_iter()
        .filter(|&item| me.stock.get(item) > 0)
        .map(|item| CargoOption {
            key: item.key(),
            image: item.image(),
            label: item.label(),
            count: me.stock.get(item),
            city_price: city_price(item),
        })
        .collect()
}

fn sale_choices(mine: &[PawnRecord], board: &BoardState, sold: &[SiteId]) -> Vec<SaleChoice> {
    let mut ticked = sold.to_vec();
    mine.iter()
        .filter_map(|pawn| {
            let price = sale_offer(pawn, board, sold)?;
            let item = pawn.item?;
            let site = pawn.location.site()?;
            let checked = !ticked.contains(&site);
            if site_by_id(site).is_some_and(|site| !buys_without_limit(site)) {
                ticked.push(site);
            }
            Some(SaleChoice {
                pawn_id: pawn.id,
                label: format!(
                    "{} {} : vendre {} pour {price} écus",
                    capitalised(pawn.kind.label()),
                    location_name(pawn.location),
                    item.with_article()
                ),
                checked,
            })
        })
        .collect()
}

fn transfer_choices(mine: &[PawnRecord]) -> Vec<TransferChoice> {
    mine.iter()
        .flat_map(|from| {
            mine.iter()
                .filter(|to| can_transfer(from, to))
                .filter_map(|to| {
                    Some(TransferChoice {
                        from_pawn_id: from.id,
                        to_pawn_id: to.id,
                        label: transfer_label(from.item?, to),
                    })
                })
        })
        .collect()
}

fn transfer_label(item: MagicItem, to: &PawnRecord) -> String {
    match to.item {
        Some(other) => format!(
            "Échanger {} contre {} de {}",
            item.with_article(),
            other.with_article(),
            to.kind.with_article()
        ),
        None => format!(
            "Passer {} {} vide",
            item.with_article(),
            to.kind.to_article()
        ),
    }
}

async fn pending_fight_view(
    conn: &mut SqliteConnection,
    game_id: i64,
    players: &[PlayerRecord],
) -> Result<Option<FightView>, AppError> {
    let Some(pending) = pending_fight(conn, game_id).await? else {
        return Ok(None);
    };
    let targets = pending
        .targets
        .iter()
        .map(|pawn| FightTarget {
            pawn_id: pawn.id,
            label: format!(
                "{} de {}",
                capitalised(pawn.kind.label()),
                players
                    .iter()
                    .find(|player| player.id == pawn.player_id)
                    .map_or("?", |player| player.name.as_str())
            ),
        })
        .collect();
    Ok(Some(FightView {
        site_id: pending.rebel.site.0,
        site: site_by_id(pending.rebel.site).map_or("?", |site| site.name),
        targets,
    }))
}
