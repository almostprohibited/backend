use axum::{
    Router,
    routing::{get, post},
};
use mongodb_connector::connector::MongoDBConnector;
use service_layers::build_service_layers;
use std::{env, net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;
use tracing::info;
use utils::logger::configure_logger;

use crate::{
    routes::{contact::contact_handler, history::history_handler, search_query::search_handler},
    structs::ServerState,
};

mod routes;
mod service_layers;
pub(crate) mod structs;

// https://nickb.dev/blog/default-musl-allocator-considered-harmful-to-performance
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[tokio::main]
async fn main() {
    configure_logger();

    let port = env::var("API_PORT").unwrap_or("3001".to_string());

    info!("Starting MongoDB client");

    let mongodb = MongoDBConnector::new().await;
    let state = Arc::new(ServerState { db: mongodb });

    let addr = format!("0.0.0.0:{port}");

    info!("MongoDB client ready");
    info!("Starting web server on: {addr}");

    let router = Router::new()
        .route("/api/search", get(search_handler))
        .route("/api/contact", post(contact_handler))
        .route("/api/history", get(history_handler));

    let type_erased_router = router.with_state(state).layer(build_service_layers());
    let service = type_erased_router.into_make_service_with_connect_info::<SocketAddr>();

    let server = TcpListener::bind(addr).await.unwrap();

    axum::serve(server, service).await.unwrap();
}
