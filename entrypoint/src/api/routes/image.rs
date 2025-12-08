use std::sync::Arc;

use crate::{ServerState, routes::error_message_erasure::ApiError};

use axum::body::Body;
use axum::debug_handler;
use axum::extract::Query;
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::extract::WithRejection;
use image_cache::ImageCache;
use reqwest::header;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Payload {
    id: String,
}

#[debug_handler]
pub(crate) async fn image_handler(
    State(state): State<Arc<ServerState>>,
    WithRejection(Query(query), _): WithRejection<Query<Payload>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(result) = state.db.find_result(query.id).await else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    let Some(image) = ImageCache::get_image(result).await else {
        return Ok(StatusCode::NOT_FOUND.into_response());
    };

    let body = Body::from(image.image);

    let headers = [(header::CONTENT_TYPE, image.mime_type)];

    Ok((headers, body).into_response())
}
