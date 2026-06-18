use std::sync::Arc;

use crate::constants::IP_HEADER;
use crate::helpers::{create_redirect, get_ip_addr, get_token};
use crate::routes::error_message_erasure::ApiError;
use crate::structs::ServerState;

use axum::debug_handler;
use axum::extract::{Path, State};
use axum::http::HeaderMap;
use axum::{http::StatusCode, response::IntoResponse};
use axum_extra::extract::WithRejection;
use common::db::ServiceType;
use common::utils::{get_backend_domain, get_current_time};
use openid_connect::providers::{get_google_oidc_provider, get_microsoft_oidc_provider};
use tracing::{debug, error};

#[debug_handler]
pub(crate) async fn notification_provider(
    headers: HeaderMap,
    State(state): State<Arc<ServerState>>,
    WithRejection(Path(path), _): WithRejection<Path<ServiceType>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(token) = get_token(&headers) else {
        debug!("Missing token");

        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    let Some(ip_addr) = get_ip_addr(&headers) else {
        error!("Request is missing {IP_HEADER} header");

        return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    };

    if state.db.find_session(&token).await.is_none() {
        debug!("Invalid token");

        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    let scopes = vec!["email".to_string()];

    let provider = match path {
        ServiceType::Google => get_google_oidc_provider(
            &format!("{}/api/notification/google/callback", get_backend_domain()),
            scopes,
        ),
        ServiceType::Microsoft => get_microsoft_oidc_provider(
            &format!(
                "{}/api/notification/microsoft/callback",
                get_backend_domain()
            ),
            scopes,
        ),
        _ => return Ok(StatusCode::NOT_IMPLEMENTED.into_response()),
    };

    let oidc_response = provider.fetch_authorization_url().await.unwrap();

    state
        .db
        .create_verification(
            &oidc_response.csrf,
            get_current_time(),
            Some(ip_addr),
            Some(oidc_response.nonce),
        )
        .await;

    Ok(create_redirect(&oidc_response.url, StatusCode::SEE_OTHER))
}
