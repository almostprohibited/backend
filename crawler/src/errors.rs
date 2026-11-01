use chromiumoxide::error::CdpError;
use reqwest::{
    StatusCode,
    header::{InvalidHeaderName, InvalidHeaderValue},
};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("Handled unhandled error: {0}")]
    UnhandledError(String),
    #[error("Unprotected middleware general error: {0}")]
    UnprotectedClientMiddlewareGeneralError(#[from] reqwest_middleware::Error),
    #[error("Unprotected crawler general error: {0}")]
    UnprotectedClientGeneralError(#[from] reqwest::Error),
    #[error("Unprotected crawler failed to create header")]
    UnprotectedClientInvalidHeader,
    #[error("Non 2xx response returned: {0}")]
    InvalidResponseCodeError(StatusCode),
    #[error("Browser protocol error {0}")]
    BrowserGeneralError(#[from] CdpError),
    #[error("Browser response contains invalid status {0}")]
    BrowserInvalidStatus(String),
    #[error("Browser response missing status")]
    BrowserMissingStatus,
    #[error("Browser response missing headers")]
    BrowserMissingHeaders,
    #[error("Browser response is missing")]
    BrowserMissingResponse,
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
