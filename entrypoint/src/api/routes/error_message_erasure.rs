use axum::{
    extract::rejection::{JsonRejection, QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use thiserror::Error;
use tracing::debug;

#[derive(Debug, Error)]
pub(crate) enum ApiError {
    #[error(transparent)]
    QueryExtractorRejection(#[from] QueryRejection),
    #[error(transparent)]
    JsonExtractorRejection(#[from] JsonRejection),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Self::QueryExtractorRejection(rejection) => (rejection.status(), rejection.body_text()),
            Self::JsonExtractorRejection(rejection) => (rejection.status(), rejection.body_text()),
        };

        debug!("Failed to parse incoming request: {}, {}", status, message);

        StatusCode::BAD_REQUEST.into_response()
    }
}
