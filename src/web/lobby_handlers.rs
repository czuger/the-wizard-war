use askama::Template;
use axum::Form;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};
use serde::Deserialize;

use crate::domain::rules::{DEFAULT_TURNS, MAX_PLAYERS, MAX_TURNS};
use crate::error::AppError;
use crate::service::{CreatedGame, create_game as create_game_service};
use crate::web::app_state::AppState;

/// The game creation form.
#[derive(Debug, Deserialize)]
pub struct CreateGameForm {
    #[serde(default)]
    player_1: String,
    #[serde(default)]
    player_2: String,
    #[serde(default)]
    player_3: String,
    #[serde(default)]
    player_4: String,
    #[serde(default)]
    player_5: String,
    #[serde(default)]
    player_6: String,
    #[serde(default)]
    max_turns: String,
}

impl CreateGameForm {
    fn fields(&self) -> [&String; MAX_PLAYERS] {
        [
            &self.player_1,
            &self.player_2,
            &self.player_3,
            &self.player_4,
            &self.player_5,
            &self.player_6,
        ]
    }

    /// Names typed in the form, skipping blank fields.
    fn player_names(&self) -> Vec<String> {
        self.fields()
            .into_iter()
            .filter(|name| !name.trim().is_empty())
            .cloned()
            .collect()
    }
}

#[derive(Template)]
#[template(path = "lobby.html")]
struct LobbyPage {
    names: Vec<String>,
    max_turns: String,
    max_turns_limit: i64,
    error: Option<String>,
}

#[derive(Template)]
#[template(path = "game_created.html")]
struct GameCreatedPage {
    game: CreatedGame,
}

/// `GET /`: the game creation form.
pub async fn show_lobby() -> Result<Html<String>, AppError> {
    let page = LobbyPage {
        names: vec![String::new(); MAX_PLAYERS],
        max_turns: DEFAULT_TURNS.to_string(),
        max_turns_limit: MAX_TURNS,
        error: None,
    };
    Ok(Html(page.render()?))
}

/// `POST /games`: creates a game and shows each player's private link.
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
    let created = match max_turns {
        Ok(turns) => create_game_service(&state.pool, &form.player_names(), turns).await,
        Err(error) => Err(error),
    };
    match created {
        Ok(game) => Ok(Html(GameCreatedPage { game }.render()?).into_response()),
        Err(error) if !error.is_internal() => {
            let page = LobbyPage {
                names: form.fields().into_iter().cloned().collect(),
                max_turns: form.max_turns.clone(),
                max_turns_limit: MAX_TURNS,
                error: Some(error.to_string()),
            };
            Ok((StatusCode::BAD_REQUEST, Html(page.render()?)).into_response())
        }
        Err(error) => Err(error),
    }
}
