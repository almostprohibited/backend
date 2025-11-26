use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::WithRejection;
use common::{result::base::CrawlResult, search_params::ApiSearchInput};
use serde::Serialize;
use tokio::time::Instant;
use tracing::debug;

use crate::{ServerState, routes::error_message_erasure::ApiError};

#[derive(Serialize, Debug)]
struct ApiResult {
    items: Vec<CrawlResult>,
    total_count: u64,
}

pub(crate) async fn search_handler(
    State(state): State<Arc<ServerState>>,
    WithRejection(Query(params), _): WithRejection<Query<ApiSearchInput>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    let start_time = Instant::now();

    debug!("{params:?}");

    let db_results = state.db.search_items(&params).await;

    // TODO: can probably delete this and just return db_results
    let result = ApiResult {
        items: db_results.items,
        total_count: db_results.total_count,
    };

    debug!("{:?}", result);

    let response: Json<ApiResult> = Json::from(result);

    debug!("Request time: {}ms", start_time.elapsed().as_millis());

    Ok(response.into_response())
}
