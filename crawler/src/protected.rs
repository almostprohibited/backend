use std::sync::Arc;

use headless_chrome::{Browser, FetcherOptions, LaunchOptionsBuilder, Tab};

use crate::{errors::CrawlerError, request::Request, traits::Crawler};

pub struct ProtectedCrawler {
    browser: Arc<Browser>,
}

impl ProtectedCrawler {
    pub fn new() -> Result<Self, CrawlerError> {
        let browser = Self::create_browser()?;

        Ok(Self {
            browser: Arc::new(browser),
        })
    }

    fn create_browser() -> Result<Browser, CrawlerError> {
        let fetcher_opts = FetcherOptions::default()
            .with_allow_download(true)
            .with_install_dir(Some("./chrome"));

        let launch_opts = LaunchOptionsBuilder::default()
            .fetcher_options(fetcher_opts)
            .build()?;

        let browser = Browser::new(launch_opts);

        Ok(browser?)
    }

    fn make_request(&self, url: &str) -> Result<Arc<Tab>, CrawlerError> {
        let tab = self.browser.new_tab()?;
        tab.navigate_to(url)?;

        Ok(tab)
    }
}

impl Crawler for ProtectedCrawler {
    async fn make_web_request(&self, request: Request) -> Result<String, CrawlerError> {
        let result = self
            .make_request(&request.url)
            .unwrap()
            .wait_for_element("body")
            .unwrap()
            .get_content();

        Ok(result?)
    }
}
