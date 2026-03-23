use std::sync::Arc;

use crate::routes::auth::utils::Provider;
use crate::{ServerState, routes::error_message_erasure::ApiError};

use axum::debug_handler;
use axum::extract::{Path, Query};
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::extract::WithRejection;
use openid_connect::providers::DiscordProvider;
use openid_connect::traits::OidcProvider;
use serde::Deserialize;
use tracing::debug;

#[derive(Deserialize, Debug)]
#[serde(deny_unknown_fields)]
pub(crate) struct Payload {
    code: String,
    state: String,
}

#[debug_handler]
pub(crate) async fn callback(
    State(_state): State<Arc<ServerState>>,
    WithRejection(Path(path), _): WithRejection<Path<Provider>, ApiError>,
    WithRejection(Query(query), _): WithRejection<Query<Payload>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    debug!("{path:?}");
    debug!("{query:?}");

    let _ = match path {
        Provider::Discord => DiscordProvider::exchange_code(query.code, query.state).await,
    };

    Ok(StatusCode::OK)
}
