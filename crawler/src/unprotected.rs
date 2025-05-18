use std::error::Error;

use reqwest::ClientBuilder;
use tracing::info;

use crate::{
    request::Request,
    traits::{Crawler, HttpMethod},
};

#[derive(Copy, Clone)]
pub struct UnprotectedCrawler {}

impl UnprotectedCrawler {
    pub fn new() -> Self {
        Self {}
    }
}

impl Crawler for UnprotectedCrawler {
    async fn make_web_request(&self, request: Request) -> Result<String, Box<dyn Error>> {
        let client = ClientBuilder::new().build()?;

        let mut request_builder = match request.method {
            HttpMethod::GET => client.get(request.url.clone()),
            HttpMethod::POST => client.post(request.url.clone()),
        };

        // set user agent here in case passed in request
        // object overrides it
        request_builder = request_builder.header("User-Agent", " Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36");

        if let Some(body) = request.body {
            request_builder = request_builder.json(&body);
        }

        if let Some(headers) = request.headers {
            for (key, value) in headers.iter() {
                request_builder = request_builder.header(key, value);
            }
        }

        info!("Sending request to {}", request.url);

        let sent_request = request_builder.send().await?;

        Ok(sent_request.text().await?)
    }
}
