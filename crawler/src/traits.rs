use async_trait::async_trait;
use reqwest::{StatusCode, header::HeaderMap};

use crate::{errors::CrawlerError, request::Request};

#[derive(Debug)]
pub enum HttpMethod {
    GET,
    POST,
}

pub struct CrawlerResponse {
    pub body: String,
    pub raw_bytes: Vec<u8>,
    pub response_code: StatusCode,
    pub headers: HeaderMap,
}

#[async_trait]
pub(crate) trait Client {
    fn set_cookie(&self, url: &str, cookie: &str);
    async fn send_request(&self, request: Request) -> Result<CrawlerResponse, CrawlerError>;
}
