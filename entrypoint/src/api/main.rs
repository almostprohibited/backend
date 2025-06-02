use axum::{
    Json, Router,
    extract::{Query, State, rejection::QueryRejection},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::get,
};
use axum_extra::extract::WithRejection;
use mongodb_connector::{connector::MongoDBConnector, stages::traits::QueryParams};
use retailers::results::firearm::FirearmResult;
use serde::Serialize;
use service_layers::build_service_layers;
use std::sync::Arc;
use thiserror::Error;
use tokio::{join, net::TcpListener, time::Instant};
use tracing::{debug, info};
use utils::logger::configure_logger;

mod service_layers;

struct ServerState {
    db: MongoDBConnector,
}

#[derive(Serialize)]
struct ApiResult {
    firearms: Vec<FirearmResult>,
    total_count: u64,
}

#[tokio::main]
async fn main() {
    configure_logger();

    info!("Starting MongoDB client");

    let state = Arc::new(ServerState {
        db: MongoDBConnector::new().await,
    });

    info!("MongoDB client ready");
    info!("Starting web server");

    let router = Router::new()
        .route("/api", get(query))
        .with_state(state)
        .layer(build_service_layers());
    let server = TcpListener::bind("0.0.0.0:3001").await.unwrap();

    axum::serve(server, router).await.unwrap();
}

#[derive(Debug, Error)]
enum ApiError {
    #[error(transparent)]
    QueryExtractorRejection(#[from] QueryRejection),
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            ApiError::QueryExtractorRejection(rejection) => {
                (rejection.status(), rejection.body_text())
            }
        };

        debug!("Failed to parse incoming request: {}, {}", status, message);

        status.into_response()
    }
}

async fn query(
    State(state): State<Arc<ServerState>>,
    WithRejection(Query(params), _): WithRejection<Query<QueryParams>, ApiError>,
) -> Result<impl IntoResponse, StatusCode> {
    let start_time = Instant::now();

    let (firearms, count) = join!(state.db.search(&params), state.db.count(&params));

    let result = ApiResult {
        firearms,
        total_count: count.total_count,
    };

    debug!("{:?}", count);

    let response: Json<ApiResult> = Json::from(result);

    debug!("Request time: {}ms", start_time.elapsed().as_millis());

    return Ok(response);
}
