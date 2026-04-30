use std::time::Duration;

use axum::http::{HeaderValue, Method, StatusCode, header::CONTENT_TYPE};
use axum_otel_metrics::{HttpMetricsLayer, HttpMetricsLayerBuilder};
use tower::{
    ServiceBuilder,
    layer::util::{Identity, Stack},
};
use tower_http::{cors::CorsLayer, timeout::TimeoutLayer};

const DEFAULT_TIMEOUT: u64 = 5;

pub(crate) fn build_service_layers()
-> ServiceBuilder<Stack<HttpMetricsLayer, Stack<TimeoutLayer, Stack<CorsLayer, Identity>>>> {
    ServiceBuilder::new()
        .layer(build_cors())
        .layer(build_timeout())
        .layer(build_metrics())
}

fn build_cors() -> CorsLayer {
    let mut cors_layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST, Method::DELETE])
        .allow_headers([CONTENT_TYPE])
        .allow_credentials(true);

    if cfg!(debug_assertions) {
        cors_layer =
            cors_layer.allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap());
    }

    cors_layer
}

fn build_timeout() -> TimeoutLayer {
    TimeoutLayer::with_status_code(
        StatusCode::GATEWAY_TIMEOUT,
        Duration::from_secs(DEFAULT_TIMEOUT),
    )
}

fn build_metrics() -> HttpMetricsLayer {
    HttpMetricsLayerBuilder::new().build()
}
