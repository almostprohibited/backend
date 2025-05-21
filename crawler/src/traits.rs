use crate::{errors::CrawlerError, request::Request};

#[derive(Debug)]
pub enum HttpMethod {
    GET,
    POST,
}

pub trait Crawler {
    fn make_web_request(
        &self,
        request: Request,
    ) -> impl Future<Output = Result<String, CrawlerError>> + Send;
}
