use axum::{
    Router,
    routing::{delete, get, post},
};
use common::utils::is_beta_environment;
use metrics::{configure_metrics, shutdown_metrics};
use mongodb_connector::connector::MongoDBConnector;
use service_layers::build_service_layers;
use std::{env, net::SocketAddr, sync::Arc};
use tokio::{
    net::TcpListener,
    select,
    signal::{
        ctrl_c,
        unix::{self, SignalKind},
    },
};
use tower_governor::{GovernorLayer, governor::GovernorConfigBuilder};
use tracing::info;
use utils::logger::configure_logger;

use crate::{
    helpers::GovernorIpExtractor,
    routes::{
        auth::{callback, delete_handler, email_login, email_otp, logout, provider},
        contact::contact_handler,
        history::history_handler,
        image::image_handler,
        notifications::{
            delete_channels, get_notification_channels, notification_add_email,
            notification_callback, notification_provider,
        },
        search_query::search_handler,
    },
    structs::ServerState,
};

pub(crate) mod constants;
pub(crate) mod helpers;
mod routes;
mod service_layers;
pub(crate) mod structs;

// https://nickb.dev/blog/default-musl-allocator-considered-harmful-to-performance
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

const DEFAULT_TPS_LIMIT: u32 = 2;
const DEFAULT_AUTH_TPS_LIMIT: u32 = 1;
const IMAGE_TPS_LIMIT: u32 = 64;

#[tokio::main]
async fn main() {
    configure_logger();
    configure_metrics();

    let port = env::var("API_PORT").unwrap_or("3001".to_string());

    info!("Starting MongoDB client");

    let mongodb = MongoDBConnector::new().await;
    let state = Arc::new(ServerState { db: mongodb });

    let addr = format!("0.0.0.0:{port}");

    let is_beta = is_beta_environment();

    info!("MongoDB client ready");
    info!("Starting web server on: {addr}");

    // the GovernorConfig contains types that are not
    // re-exported, I can't write a return type for
    // a normal function unless I also depend on the
    // same libs as they do
    let get_governor = |tps: u32| {
        GovernorConfigBuilder::default()
            .per_millisecond(1000 / tps as u64)
            .burst_size(tps)
            .use_headers()
            .key_extractor(GovernorIpExtractor)
            .finish()
            .unwrap()
    };

    let mut router = Router::new()
        .route(
            "/api/search",
            get(search_handler).route_layer(GovernorLayer::new(get_governor(DEFAULT_TPS_LIMIT))),
        )
        .route(
            "/api/contact",
            post(contact_handler).route_layer(GovernorLayer::new(get_governor(DEFAULT_TPS_LIMIT))),
        )
        .route(
            "/api/history",
            get(history_handler).route_layer(GovernorLayer::new(get_governor(DEFAULT_TPS_LIMIT))),
        )
        .route(
            "/api/image",
            get(image_handler).route_layer(GovernorLayer::new(get_governor(IMAGE_TPS_LIMIT))),
        );

    if is_beta {
        router = router
            .route(
                "/api/auth/{provider}/provider",
                post(provider)
                    .route_layer(GovernorLayer::new(get_governor(DEFAULT_AUTH_TPS_LIMIT))),
            )
            .route(
                "/api/auth/{provider}/callback",
                get(callback).route_layer(GovernorLayer::new(get_governor(DEFAULT_AUTH_TPS_LIMIT))),
            )
            .route(
                "/api/auth/email/login",
                post(email_login)
                    .route_layer(GovernorLayer::new(get_governor(DEFAULT_AUTH_TPS_LIMIT))),
            )
            .route(
                "/api/auth/email/otp",
                post(email_otp)
                    .route_layer(GovernorLayer::new(get_governor(DEFAULT_AUTH_TPS_LIMIT))),
            )
            .route(
                "/api/auth/logout",
                delete(logout)
                    .route_layer(GovernorLayer::new(get_governor(DEFAULT_AUTH_TPS_LIMIT))),
            )
            .route(
                "/api/auth/delete",
                delete(delete_handler)
                    .route_layer(GovernorLayer::new(get_governor(DEFAULT_AUTH_TPS_LIMIT))),
            )
            .route(
                "/api/notification/{provider}/provider",
                post(notification_provider)
                    .route_layer(GovernorLayer::new(get_governor(DEFAULT_AUTH_TPS_LIMIT))),
            )
            .route(
                "/api/notification/{provider}/callback",
                get(notification_callback)
                    .route_layer(GovernorLayer::new(get_governor(DEFAULT_AUTH_TPS_LIMIT))),
            )
            .route(
                "/api/notification/channels",
                get(get_notification_channels)
                    .route_layer(GovernorLayer::new(get_governor(DEFAULT_TPS_LIMIT))),
            )
            .route(
                "/api/notification/email",
                post(notification_add_email)
                    .route_layer(GovernorLayer::new(get_governor(DEFAULT_AUTH_TPS_LIMIT))),
            )
            .route(
                "/api/notification/delete",
                delete(delete_channels)
                    .route_layer(GovernorLayer::new(get_governor(DEFAULT_AUTH_TPS_LIMIT))),
            );
    }

    let type_erased_router = router.with_state(state).layer(build_service_layers());
    let service = type_erased_router.into_make_service_with_connect_info::<SocketAddr>();

    let server = TcpListener::bind(addr).await.unwrap();

    info!("is beta={is_beta}");
    info!("is development={}", cfg!(debug_assertions));

    axum::serve(server, service)
        .with_graceful_shutdown(shutdown_signal())
        .await
        .unwrap();

    shutdown_metrics();
}

async fn shutdown_signal() {
    select! {
        _ = async {ctrl_c().await.unwrap()} => {},

        // sorry Windows people, not dealing with windows related sigs
        _ = async {unix::signal(SignalKind::terminate()).unwrap().recv().await} => {},
    };
}
