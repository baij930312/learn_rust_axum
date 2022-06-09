use axum::response::{IntoResponse, Response};
use axum::http::StatusCode;

#[derive(Debug)]
pub enum CustomError {
    Database(String),
}

// So that errors get printed to the browser?
impl IntoResponse for CustomError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            CustomError::Database(message) => (StatusCode::UNPROCESSABLE_ENTITY, message),
        };

        format!("status = {}, message = {}", status, error_message).into_response()
    }
}

// Any errors from sqlx get converted to CustomError
impl From<sqlx::Error> for CustomError {
    fn from(err: sqlx::Error) -> CustomError {
        CustomError::Database(err.to_string())
    }
}