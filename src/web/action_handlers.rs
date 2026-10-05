use std::collections::HashMap;

use askama::Template;
use axum::Form;
use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::domain::{
    Conversion, ConversionError, ItemCounts, Location, MagicItem, PawnOrder, Purchase,
    PurchaseError, SiteId,
};
use crate::error::AppError;
use crate::service::{
    GameView, RebelMove, announce_fanatic, attack_pawn, buy_from_bank, convert_site, move_pawns,
    move_rebels, pay_expense, produce_items, sell_in_city, sell_items,
};
use crate::web::app_state::AppState;
use crate::web::game_page::GamePage;

/// The announcement form: a fanatic id, or an empty value to send none.
#[derive(Debug, Deserialize)]
pub struct AnnouncementForm {
    #[serde(default)]
    fanatic: String,
}

/// The expense form.
#[derive(Debug, Deserialize)]
pub struct ExpenseForm {
    #[serde(default)]
    amount: String,
}

/// The production form: how many items of each kind to take.
#[derive(Debug, Deserialize)]
pub struct ProductionForm {
    #[serde(default)]
    toratsa: String,
    #[serde(default)]
    xephis: String,
    #[serde(default)]
    yaboul: String,
}

/// The conversion form: a value made by [`Conversion::form_value`], or empty to pass.
#[derive(Debug, Deserialize)]
pub struct ConversionForm {
    #[serde(default)]
    conversion: String,
}

/// The fight form: the pawn the rebels attack.
#[derive(Debug, Deserialize)]
pub struct FightForm {
    #[serde(default)]
    pawn: String,
}

/// The city sale form: an item key, or an empty value to pass.
#[derive(Debug, Deserialize)]
pub struct CitySaleForm {
    #[serde(default)]
    item: String,
}

/// The purchase form: how many of each thing to buy or sell back.
#[derive(Debug, Deserialize)]
pub struct PurchaseForm {
    #[serde(default)]
    peddlers: String,
    #[serde(default)]
    caravans: String,
    #[serde(default)]
    laboratories: String,
    #[serde(default)]
    sold_peddlers: String,
    #[serde(default)]
    sold_caravans: String,
}

/// Prefix of the rebel form's fields, followed by the pawn's id; the value is a site id.
const REBEL_FIELD_PREFIX: &str = "rebel_";

/// Prefix of the pawn form's destination fields, followed by the pawn's id; the value is a
/// [`Location::form_value`], or empty to stay.
const PAWN_FIELD_PREFIX: &str = "pawn_";

/// Prefix of the pawn form's cargo fields, followed by the pawn's id; the value is an item key.
const ITEM_FIELD_PREFIX: &str = "item_";

/// Prefix of the sale form's checkboxes, followed by the pawn's id.
const SELL_FIELD_PREFIX: &str = "sell_";

/// `POST …/announcement`
pub async fn announce(
    State(state): State<AppState>,
    Path((game_id, token)): Path<(i64, String)>,
    Form(form): Form<AnnouncementForm>,
) -> Result<Response, AppError> {
    let result = match form.fanatic.trim() {
        "" => Ok(None),
        id => id
            .parse()
            .map(Some)
            .map_err(|_| AppError::FanaticNotAvailable),
    };
    let result = match result {
        Ok(fanatic_id) => announce_fanatic(&state.pool, game_id, &token, fanatic_id).await,
        Err(error) => Err(error),
    };
    respond(&state, game_id, &token, result).await
}

/// `POST …/expense`
pub async fn pay(
    State(state): State<AppState>,
    Path((game_id, token)): Path<(i64, String)>,
    Form(form): Form<ExpenseForm>,
) -> Result<Response, AppError> {
    let result = match parse_count(&form.amount, AppError::InvalidExpense) {
        Ok(amount) => pay_expense(&state.pool, game_id, &token, amount).await,
        Err(error) => Err(error),
    };
    respond(&state, game_id, &token, result).await
}

/// `POST …/production`
pub async fn produce(
    State(state): State<AppState>,
    Path((game_id, token)): Path<(i64, String)>,
    Form(form): Form<ProductionForm>,
) -> Result<Response, AppError> {
    let result = match parse_items(&form) {
        Ok(items) => produce_items(&state.pool, game_id, &token, items).await,
        Err(error) => Err(error),
    };
    respond(&state, game_id, &token, result).await
}

/// `POST …/conversion`
pub async fn convert(
    State(state): State<AppState>,
    Path((game_id, token)): Path<(i64, String)>,
    Form(form): Form<ConversionForm>,
) -> Result<Response, AppError> {
    let result = match form.conversion.trim() {
        "" => Ok(None),
        value => Conversion::from_form_value(value)
            .map(Some)
            .ok_or(AppError::Conversion(ConversionError::UnknownSite)),
    };
    let result = match result {
        Ok(conversion) => convert_site(&state.pool, game_id, &token, conversion).await,
        Err(error) => Err(error),
    };
    respond(&state, game_id, &token, result).await
}

/// `POST …/rebels`
pub async fn move_rebel_pawns(
    State(state): State<AppState>,
    Path((game_id, token)): Path<(i64, String)>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Response, AppError> {
    let result = match parse_rebel_moves(&form) {
        Ok(moves) => move_rebels(&state.pool, game_id, &token, moves).await,
        Err(error) => Err(error),
    };
    respond(&state, game_id, &token, result).await
}

/// `POST …/fight`
pub async fn fight(
    State(state): State<AppState>,
    Path((game_id, token)): Path<(i64, String)>,
    Form(form): Form<FightForm>,
) -> Result<Response, AppError> {
    let result = match form.pawn.trim().parse() {
        Ok(pawn_id) => attack_pawn(&state.pool, game_id, &token, pawn_id).await,
        Err(_) => Err(AppError::InvalidFightTarget),
    };
    respond(&state, game_id, &token, result).await
}

/// `POST …/pawns`
pub async fn move_pawn_tokens(
    State(state): State<AppState>,
    Path((game_id, token)): Path<(i64, String)>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Response, AppError> {
    let result = match parse_pawn_orders(&form) {
        Ok(orders) => move_pawns(&state.pool, game_id, &token, orders).await,
        Err(error) => Err(error),
    };
    respond(&state, game_id, &token, result).await
}

/// `POST …/sales`
pub async fn sell(
    State(state): State<AppState>,
    Path((game_id, token)): Path<(i64, String)>,
    Form(form): Form<HashMap<String, String>>,
) -> Result<Response, AppError> {
    let pawn_ids: Result<Vec<i64>, AppError> = form
        .keys()
        .filter_map(|key| key.strip_prefix(SELL_FIELD_PREFIX))
        .map(|id| id.parse().map_err(|_| AppError::InvalidSale))
        .collect();
    let result = match pawn_ids {
        Ok(mut ids) => {
            ids.sort_unstable();
            sell_items(&state.pool, game_id, &token, ids).await
        }
        Err(error) => Err(error),
    };
    respond(&state, game_id, &token, result).await
}

/// `POST …/city-sale`
pub async fn sell_city(
    State(state): State<AppState>,
    Path((game_id, token)): Path<(i64, String)>,
    Form(form): Form<CitySaleForm>,
) -> Result<Response, AppError> {
    let item = match form.item.trim() {
        "" => Ok(None),
        key => MagicItem::from_key(key)
            .map(Some)
            .ok_or(AppError::NotInStock),
    };
    let result = match item {
        Ok(item) => sell_in_city(&state.pool, game_id, &token, item).await,
        Err(error) => Err(error),
    };
    respond(&state, game_id, &token, result).await
}

/// `POST …/purchase`
pub async fn purchase(
    State(state): State<AppState>,
    Path((game_id, token)): Path<(i64, String)>,
    Form(form): Form<PurchaseForm>,
) -> Result<Response, AppError> {
    let result = match parse_purchase(&form) {
        Ok(purchase) => buy_from_bank(&state.pool, game_id, &token, purchase).await,
        Err(error) => Err(error),
    };
    respond(&state, game_id, &token, result).await
}

fn parse_pawn_orders(form: &HashMap<String, String>) -> Result<Vec<PawnOrder>, AppError> {
    let mut orders: Vec<PawnOrder> = form
        .iter()
        .filter_map(|(key, value)| Some((key.strip_prefix(PAWN_FIELD_PREFIX)?, value.trim())))
        .filter(|(_, value)| !value.is_empty())
        .map(|(id, value)| {
            let pawn_id: i64 = id.parse().map_err(|_| AppError::InvalidPawnMove)?;
            let to = Location::from_form_value(value).ok_or(AppError::InvalidPawnMove)?;
            let item = form
                .get(&format!("{ITEM_FIELD_PREFIX}{pawn_id}"))
                .map(|key| key.trim())
                .filter(|key| !key.is_empty())
                .map(|key| MagicItem::from_key(key).ok_or(AppError::NotInStock))
                .transpose()?;
            Ok(PawnOrder { pawn_id, to, item })
        })
        .collect::<Result<_, AppError>>()?;
    orders.sort_by_key(|order| order.pawn_id);
    Ok(orders)
}

fn parse_purchase(form: &PurchaseForm) -> Result<Purchase, AppError> {
    let count = |value: &str| parse_count(value, AppError::Purchase(PurchaseError::Negative));
    Ok(Purchase {
        peddlers: count(&form.peddlers)?,
        caravans: count(&form.caravans)?,
        laboratories: count(&form.laboratories)?,
        sold_peddlers: count(&form.sold_peddlers)?,
        sold_caravans: count(&form.sold_caravans)?,
    })
}

fn parse_rebel_moves(form: &HashMap<String, String>) -> Result<Vec<RebelMove>, AppError> {
    form.iter()
        .filter_map(|(key, value)| Some((key.strip_prefix(REBEL_FIELD_PREFIX)?, value)))
        .map(|(id, site)| {
            let rebel_id = id.parse().map_err(|_| AppError::InvalidRebelMove)?;
            let site: u8 = site
                .trim()
                .parse()
                .map_err(|_| AppError::InvalidRebelMove)?;
            Ok(RebelMove {
                rebel_id,
                to: SiteId(site),
            })
        })
        .collect()
}

fn parse_count(value: &str, error: AppError) -> Result<i64, AppError> {
    match value.trim() {
        "" => Ok(0),
        text => text.parse().map_err(|_| error),
    }
}

fn parse_items(form: &ProductionForm) -> Result<ItemCounts, AppError> {
    Ok(ItemCounts::new(
        parse_count(&form.toratsa, AppError::SupplyExhausted)?,
        parse_count(&form.xephis, AppError::SupplyExhausted)?,
        parse_count(&form.yaboul, AppError::SupplyExhausted)?,
    ))
}

/// Redirects back to the player's page, or shows it with the error when the action was refused.
async fn respond(
    state: &AppState,
    game_id: i64,
    token: &str,
    result: Result<(), AppError>,
) -> Result<Response, AppError> {
    match result {
        Ok(()) => Ok(Redirect::to(&format!("/games/{game_id}/players/{token}")).into_response()),
        Err(error) if error.is_internal() => Err(error),
        Err(error) => {
            let view = GameView::load(&state.pool, game_id, token).await?;
            let page = GamePage::new(view, Some(error.to_string()));
            Ok((StatusCode::BAD_REQUEST, Html(page.render()?)).into_response())
        }
    }
}
