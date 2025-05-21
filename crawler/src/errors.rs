use thiserror::Error;

#[derive(Error, Debug)]
pub enum CrawlerError {
    #[error("Unprotected crawler general error")]
    UnprotectedClientGeneralError(#[from] reqwest::Error),
    #[error("Unprotected crawler failed to create header")]
    UnprotectedClientInvalidHeader,
}

impl From<reqwest::header::InvalidHeaderName> for CrawlerError {
    fn from(_err: reqwest::header::InvalidHeaderName) -> Self {
        Self::UnprotectedClientInvalidHeader
    }
}

impl From<reqwest::header::InvalidHeaderValue> for CrawlerError {
    fn from(_err: reqwest::header::InvalidHeaderValue) -> Self {
        Self::UnprotectedClientInvalidHeader
    }
}
