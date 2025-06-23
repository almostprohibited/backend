use std::sync::Arc;

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::extract::WithRejection;
use common::result::base::CrawlResult;
use mongodb_connector::stages::traits::QueryParams;
use serde::Serialize;
use tokio::{join, time::Instant};
use tracing::debug;

use crate::{ServerState, routes::error_message_erasure::ApiError};

#[derive(Serialize, Debug)]
struct ApiResult {
    items: Vec<CrawlResult>,
    total_count: u64,
}

pub(crate) async fn search_handler(
    State(state): State<Arc<ServerState>>,
    WithRejection(Query(params), _): WithRejection<Query<QueryParams>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    let start_time = Instant::now();

    let (results, count) = join!(
        state.db.search_items(&params),
        state.db.count_items(&params)
    );

    let result = ApiResult {
        items: results,
        total_count: count.total_count,
    };

    debug!("{:?}", result);

    let response: Json<ApiResult> = Json::from(result);

    debug!("Request time: {}ms", start_time.elapsed().as_millis());

    return Ok(response);
}
