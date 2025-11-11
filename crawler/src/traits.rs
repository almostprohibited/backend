use reqwest::header::HeaderMap;

#[derive(Debug)]
pub enum HttpMethod {
    GET,
    POST,
}

pub struct CrawlerResponse {
    pub body: String,
    pub headers: HeaderMap,
}
