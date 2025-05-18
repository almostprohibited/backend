use std::{collections::HashMap, sync::Arc};

use axum::{
    Json, Router,
    extract::{Query, State},
    http::{HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::get,
};
use mongodb_connector::connector::MongoDBConnector;
use retailers::results::firearm::FirearmResult;
use tokio::{net::TcpListener, time::Instant};
use tower::ServiceBuilder;
use tower_http::cors::CorsLayer;
use tracing::{debug, info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

struct ServerState {
    db: MongoDBConnector,
}

#[tokio::main]
async fn main() {
    let env_log = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .expect("Failed to create tracing filter");

    let subscriber = FmtSubscriber::builder()
        .pretty()
        .compact()
        .with_file(false)
        .with_env_filter(env_log);

    tracing::subscriber::set_global_default(subscriber.finish())
        .expect("Failed to create log subscription");

    info!("Starting MongoDB client");

    let state = Arc::new(ServerState {
        db: MongoDBConnector::new().await,
    });

    info!("MongoDB client ready");
    info!("Starting web server");

    let cors_layer = CorsLayer::new()
        .allow_methods([Method::GET])
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap());
    let service_layer = ServiceBuilder::new().layer(cors_layer);

    let router = Router::new()
        .route("/api", get(query))
        .with_state(state)
        .layer(service_layer);
    let server = TcpListener::bind("0.0.0.0:3001").await.unwrap();

    axum::serve(server, router).await.unwrap();
}

async fn query(
    State(state): State<Arc<ServerState>>,
    Query(params): Query<HashMap<String, String>>,
) -> Result<impl IntoResponse, StatusCode> {
    let Some(query_string) = params.get("query") else {
        return Err(StatusCode::BAD_REQUEST);
    };

    if params.len() > 1 {
        return Err(StatusCode::BAD_REQUEST);
    }

    let start_time = Instant::now();

    let firearms = state.db.search(query_string).await;
    let response: Json<Vec<FirearmResult>> = Json::from(firearms);

    debug!("Request time: {}ms", start_time.elapsed().as_millis());

    return Ok(response);
}
