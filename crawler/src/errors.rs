use reqwest::header::{InvalidHeaderName, InvalidHeaderValue};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("Unprotected crawler general error")]
    UnprotectedClientMiddlewareGeneralError(#[from] reqwest_middleware::Error),
    #[error("Unprotected crawler general error")]
    UnprotectedClientGeneralError(#[from] reqwest::Error),
    #[error("Unprotected crawler failed to create header")]
    UnprotectedClientInvalidHeader,
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
