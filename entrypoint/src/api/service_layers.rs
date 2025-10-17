use axum::http::{HeaderValue, Method};
use reqwest::header::CONTENT_TYPE;
use tower::{
    ServiceBuilder,
    layer::util::{Identity, Stack},
};
use tower_http::cors::CorsLayer;

pub(crate) fn build_service_layers() -> ServiceBuilder<Stack<CorsLayer, Identity>> {
    let mut cors_layer = CorsLayer::new()
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([CONTENT_TYPE]);

    if cfg!(debug_assertions) {
        cors_layer =
            cors_layer.allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap());
    }

    ServiceBuilder::new().layer(cors_layer)
}
