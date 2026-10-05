use askama::Template;
use axum::http::StatusCode;
use axum::response::{Html, IntoResponse, Response};

use crate::error::AppError;

const INTERNAL_MESSAGE: &str = "Une erreur interne est survenue. Veuillez réessayer.";

#[derive(Template)]
#[template(path = "error.html")]
struct ErrorPage<'a> {
    message: &'a str,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            error if error.is_internal() => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::GameNotFound | AppError::PlayerNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::BAD_REQUEST,
        };
        let message = if self.is_internal() {
            tracing::error!(error = ?self, "request failed");
            INTERNAL_MESSAGE.to_string()
        } else {
            self.to_string()
        };
        match (ErrorPage { message: &message }).render() {
            Ok(page) => (status, Html(page)).into_response(),
            Err(error) => {
                tracing::error!(?error, "error page rendering failed");
                (status, message).into_response()
            }
        }
    }
}
