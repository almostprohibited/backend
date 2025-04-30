use std::error::Error;

use crate::request::Request;

pub enum HttpMethod {
    GET,
    POST,
}

pub trait Crawler {
    fn make_web_request(
        &self,
        request: Request,
    ) -> impl Future<Output = Result<String, Box<dyn Error>>> + Send;
}
