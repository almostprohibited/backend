use reqwest::{
    StatusCode,
    header::{InvalidHeaderName, InvalidHeaderValue},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("Unprotected middleware general error: {0}")]
    UnprotectedClientMiddlewareGeneralError(#[from] reqwest_middleware::Error),
    #[error("Unprotected crawler general error: {0}")]
    UnprotectedClientGeneralError(#[from] reqwest::Error),
    #[error("Unprotected crawler failed to create header")]
    UnprotectedClientInvalidHeader,
    #[error("Non 2xx response returned: {0}")]
    InvalidResponseCodeError(StatusCode),
}

impl From<InvalidHeaderName> for CrawlerError {
    fn from(_err: InvalidHeaderName) -> Self {
        Self::UnprotectedClientInvalidHeader
    }
}

impl From<InvalidHeaderValue> for CrawlerError {
    fn from(_err: InvalidHeaderValue) -> Self {
        Self::UnprotectedClientInvalidHeader
    }
}
