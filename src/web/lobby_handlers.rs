use askama::Template;
use axum::Form;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::domain::District;
use crate::domain::rules::{AI_NAMES, DEFAULT_TURNS, MAX_TURNS};
use crate::error::AppError;
use crate::service::create_game as create_game_service;
use crate::web::app_state::AppState;

/// The game creation form.
#[derive(Debug, Deserialize)]
pub struct CreateGameForm {
    #[serde(default)]
    name: String,
    #[serde(default)]
    max_turns: String,
    #[serde(default)]
    district: String,
}

/// A district offered on the creation form.
struct DistrictOption {
    key: &'static str,
    label: &'static str,
    checked: bool,
}

#[derive(Template)]
#[template(path = "lobby.html")]
struct LobbyPage {
    name: String,
    max_turns: String,
    max_turns_limit: i64,
    ai_names: &'static [&'static str],
    districts: Vec<DistrictOption>,
    error: Option<String>,
}

impl LobbyPage {
    fn new(name: String, max_turns: String, district: &str, error: Option<String>) -> LobbyPage {
        let chosen = District::from_key(district).unwrap_or(District::Blue);
        LobbyPage {
            name,
            max_turns,
            max_turns_limit: MAX_TURNS,
            ai_names: &AI_NAMES,
            districts: District::ALL
                .into_iter()
                .map(|district| DistrictOption {
                    key: district.key(),
                    label: district.label(),
                    checked: district == chosen,
                })
                .collect(),
            error,
        }
    }
}

/// `GET /`: the game creation form.
pub async fn show_lobby() -> Result<Html<String>, AppError> {
    let page = LobbyPage::new(String::new(), DEFAULT_TURNS.to_string(), "", None);
    Ok(Html(page.render()?))
}

/// `POST /games`: creates a game against the AIs and sends the player to their page.
///
/// Invalid input shows the form again with the error and the typed values.
pub async fn create_game(
    State(state): State<AppState>,
    Form(form): Form<CreateGameForm>,
) -> Result<Response, AppError> {
    let max_turns = form
        .max_turns
        .trim()
        .parse()
        .map_err(|_| AppError::InvalidTurnCount);
    let district = District::from_key(form.district.trim()).ok_or(AppError::InvalidDistrict);
    let created = match (max_turns, district) {
        (Ok(turns), Ok(district)) => {
            create_game_service(&state.pool, &form.name, turns, district).await
        }
        (Err(error), _) | (_, Err(error)) => Err(error),
    };
    match created {
        Ok(game) => {
            let path = format!("/games/{}/players/{}", game.game_id, game.human_token);
            Ok(Redirect::to(&path).into_response())
        }
        Err(error) if !error.is_internal() => {
            let page = LobbyPage::new(
                form.name,
                form.max_turns,
                &form.district,
                Some(error.to_string()),
            );
            Ok((StatusCode::BAD_REQUEST, Html(page.render()?)).into_response())
        }
        Err(error) => Err(error),
    }
}
