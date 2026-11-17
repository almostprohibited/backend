use std::collections::HashMap;
use std::sync::Arc;

use crate::ServerState;

use axum::{Json, debug_handler};
use axum::{extract::State, http::StatusCode, response::IntoResponse};
use common::result::enums::RetailerName;
use common::retailer_status::RetailerStatus;
use serde::Serialize;

#[derive(Serialize)]
struct ApiRetailerStatusCount {
    firearms: u64,
    other: u64,
    ammo: u64,
}

#[derive(Serialize)]
struct ApiRetailerStatus {
    counts: ApiRetailerStatusCount,
    successful: bool,
    timestamp: u64,
}

impl From<RetailerStatus> for ApiRetailerStatus {
    fn from(internal: RetailerStatus) -> Self {
        Self {
            counts: ApiRetailerStatusCount {
                firearms: internal.counts.firearms,
                other: internal.counts.other,
                ammo: internal.counts.ammo + internal.counts.ammo_no_count,
            },
            successful: internal.successful,
            timestamp: internal.timestamp.timestamp() as u64,
        }
    }
}

#[debug_handler]
pub(crate) async fn retailer_status_handler(
    State(state): State<Arc<ServerState>>,
) -> Result<impl IntoResponse, StatusCode> {
    let mut map: HashMap<RetailerName, ApiRetailerStatus> = Default::default();

    state
        .db
        .get_retailers_latest_status()
        .await
        .into_iter()
        .for_each(|internal| {
            let _ = map.insert(internal.retailer, ApiRetailerStatus::from(internal));
        });

    Ok(Json::from(map).into_response())
}
