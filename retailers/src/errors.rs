use crawler::errors::CrawlerError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RetailerError {
    #[error("Failed to deserialize JSON string into Value: {0}")]
    InvalidRequestBody(String),
    #[error("Failed to make API call")]
    CrawlerInitFailed(#[from] CrawlerError),
    #[error("Failed to parse price into u64: {0}")]
    InvalidNumber(String),
    #[error("API request is missing key in JSON response: {0}")]
    ApiResponseMissingKey(String),
    #[error("API request has wrong shape: {0}")]
    ApiResponseInvalidShape(String),
    #[error("Missing attribute {0} from element {0}")]
    HtmlElementMissingAttribute(String, String),
    #[error("Missing element {0} from HTML")]
    HtmlMissingElement(String),
    #[error("General error: {0}")]
    GeneralError(String),
    #[error("Failed to deserialize JSON string into Value {0}")]
    InvalidApiResponse(#[from] serde_json::Error),
}
