use std::sync::Arc;

use crate::constants::IP_HEADER;
use crate::helpers::{create_redirect, get_ip_addr, get_token};
use crate::routes::error_message_erasure::ApiError;
use crate::structs::ServerState;

use axum::debug_handler;
use axum::extract::{Path, Query, State};
use axum::http::HeaderMap;
use axum::{http::StatusCode, response::IntoResponse};
use axum_extra::extract::WithRejection;
use common::db::ServiceType;
use common::utils::{get_backend_domain, get_frontend_domain};
use openid_connect::providers::{get_google_oidc_provider, get_microsoft_oidc_provider};
use serde::Deserialize;
use tracing::{debug, error};

#[derive(Deserialize, Debug)]
pub(crate) struct Payload {
    state: String,
    error: Option<String>,
    code: Option<String>,
}

// TODO: better error handle the hand back to frontend
// eg. error messages if IP mismatch, if DB entry missing, etc
#[debug_handler]
pub(crate) async fn notification_callback(
    headers: HeaderMap,
    State(state): State<Arc<ServerState>>,
    WithRejection(Path(path), _): WithRejection<Path<ServiceType>, ApiError>,
    WithRejection(Query(query), _): WithRejection<Query<Payload>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(token) = get_token(&headers) else {
        debug!("Missing token");

        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    let Some(session) = state.db.find_session(&token).await else {
        debug!("Invalid token");

        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    // this should only happen if someone is messing with the API
    if query.error.clone().xor(query.code.clone()).is_none() {
        error!("Request is missing one of error or code params");
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    let Some(ip_addr) = get_ip_addr(&headers) else {
        error!("Request is missing {IP_HEADER} header");

        return Ok(StatusCode::INTERNAL_SERVER_ERROR.into_response());
    };

    // again, the error only happens if someone is messing with the API
    let provider = match path {
        ServiceType::Google => get_google_oidc_provider(
            &format!("{}/api/notification/google/callback", get_backend_domain()),
            vec![],
        ),
        ServiceType::Microsoft => get_microsoft_oidc_provider(
            &format!(
                "{}/api/notification/microsoft/callback",
                get_backend_domain()
            ),
            vec![],
        ),
        _ => return Ok(StatusCode::NOT_IMPLEMENTED.into_response()),
    };

    let Some(db_verification) = state.db.get_verification(&query.state).await else {
        error!("Saved verification code does exist");

        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    state.db.delete_verification(&query.state).await;

    if let Some(error) = query.error {
        if error == "access_denied" {
            return Ok(create_redirect(
                &format!("{}/notifications", get_frontend_domain()),
                StatusCode::TEMPORARY_REDIRECT,
            ));
        } else {
            return Ok(StatusCode::BAD_REQUEST.into_response());
        }
    }

    if let Some(saved_ip) = db_verification.ip_addr
        && saved_ip != ip_addr
    {
        error!("Saved verification does not match incoming IP address");

        return Ok(StatusCode::FORBIDDEN.into_response());
    }

    let Some(saved_nonce) = db_verification.nonce else {
        error!("Saved verification does not contain nonce");

        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    // code should exist at this point
    let code = query.code.unwrap();

    let claims = provider.exchange_code(&code, &saved_nonce).await.unwrap();

    let Some(email) = claims.email else {
        error!("Claim does not contain email");

        return Ok(StatusCode::BAD_REQUEST.into_response());
    };

    debug!(
        "create valid channel with {:?} against {:?}",
        email, session.user_id
    );

    for alert in state.db.get_notification_channels(session.user_id).await {
        if alert.identifier == email && alert.user_id == session.user_id {
            error!("Channel already exists");

            return Ok(create_redirect(
                &format!("{}/notifications", get_frontend_domain()),
                StatusCode::TEMPORARY_REDIRECT,
            ));
        }
    }

    state
        .db
        .create_notification_channel(
            &email,
            session.user_id,
            path,
            common::db::VerificationStatus::Verified,
        )
        .await;

    Ok(create_redirect(
        &format!("{}/notifications", get_frontend_domain()),
        StatusCode::TEMPORARY_REDIRECT,
    ))
}
