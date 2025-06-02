use axum::http::{HeaderValue, Method};
use tower::{
    ServiceBuilder,
    layer::util::{Identity, Stack},
};
use tower_http::cors::CorsLayer;

pub(crate) fn build_service_layers() -> ServiceBuilder<Stack<CorsLayer, Identity>> {
    let cors_layer = CorsLayer::new()
        .allow_methods([Method::GET])
        .allow_origin("http://localhost:3000".parse::<HeaderValue>().unwrap());

    ServiceBuilder::new().layer(cors_layer)
}
