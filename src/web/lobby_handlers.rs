use askama::Template;
use axum::Form;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Redirect, Response};
use serde::Deserialize;

use crate::domain::rules::{AI_NAMES, DEFAULT_TURNS, MAX_TURNS};
use crate::domain::{AiLevel, District};
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
    #[serde(default)]
    ai_level: String,
}

/// A level of the computer-controlled alchemists offered on the creation form.
struct AiLevelOption {
    key: &'static str,
    label: &'static str,
    checked: bool,
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
    ai_levels: Vec<AiLevelOption>,
    error: Option<String>,
}

/// What the player typed on the creation form, shown again after an error.
struct LobbyInput<'a> {
    name: String,
    max_turns: String,
    district: &'a str,
    ai_level: &'a str,
}

impl LobbyPage {
    /// The form as first shown, with the default choices.
    fn blank() -> LobbyPage {
        let input = LobbyInput {
            name: String::new(),
            max_turns: DEFAULT_TURNS.to_string(),
            district: "",
            ai_level: "",
        };
        LobbyPage::new(input, None)
    }

    fn new(input: LobbyInput<'_>, error: Option<String>) -> LobbyPage {
        let chosen = District::from_key(input.district).unwrap_or(District::Blue);
        let chosen_level = AiLevel::from_key(input.ai_level).unwrap_or_default();
        LobbyPage {
            name: input.name,
            max_turns: input.max_turns,
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
            ai_levels: AiLevel::ALL
                .into_iter()
                .map(|level| AiLevelOption {
                    key: level.key(),
                    label: level.label(),
                    checked: level == chosen_level,
                })
                .collect(),
            error,
        }
    }
}

/// `GET /`: the game creation form.
pub async fn show_lobby() -> Result<Html<String>, AppError> {
    Ok(Html(LobbyPage::blank().render()?))
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
    let ai_level = AiLevel::from_key(form.ai_level.trim()).ok_or(AppError::InvalidAiLevel);
    let created = match (max_turns, district, ai_level) {
        (Ok(turns), Ok(district), Ok(ai_level)) => {
            create_game_service(&state.pool, &form.name, turns, district, ai_level).await
        }
        (Err(error), _, _) | (_, Err(error), _) | (_, _, Err(error)) => Err(error),
    };
    match created {
        Ok(game) => {
            let path = format!("/games/{}/players/{}", game.game_id, game.human_token);
            Ok(Redirect::to(&path).into_response())
        }
        Err(error) if !error.is_internal() => {
            let input = LobbyInput {
                name: form.name,
                max_turns: form.max_turns,
                district: &form.district,
                ai_level: &form.ai_level,
            };
            let page = LobbyPage::new(input, Some(error.to_string()));
            Ok((StatusCode::BAD_REQUEST, Html(page.render()?)).into_response())
        }
        Err(error) => Err(error),
    }
}

#[cfg(test)]
mod tests {
    use axum::body::to_bytes;

    use super::*;
    use crate::db;

    fn form(ai_level: &str) -> CreateGameForm {
        CreateGameForm {
            name: "Ana".to_string(),
            max_turns: "8".to_string(),
            district: "green".to_string(),
            ai_level: ai_level.to_string(),
        }
    }

    #[test]
    fn the_form_offers_every_level_beginner_first() {
        let html = LobbyPage::blank().render().unwrap();
        assert!(html.contains(r#"value="beginner" checked"#));
        assert!(html.contains(r#"value="strong""#));
        assert!(html.contains("Débutant") && html.contains("Fort"));
    }

    #[tokio::test]
    async fn an_unknown_level_shows_the_form_again() {
        let pool = db::connect("sqlite::memory:").await.unwrap();
        let state = AppState { pool };
        let response = create_game(State(state.clone()), Form(form("expert")))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let html = String::from_utf8(body.to_vec()).unwrap();
        assert!(html.contains("Niveau des alchimistes inconnu."));

        let response = create_game(State(state), Form(form("strong")))
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::SEE_OTHER);
    }
}
