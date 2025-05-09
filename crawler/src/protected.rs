use std::{error::Error, sync::Arc};

use headless_chrome::{Browser, FetcherOptions, LaunchOptionsBuilder, Tab};

use crate::{request::Request, traits::Crawler};

pub struct ProtectedCrawler {
    browser: Arc<Browser>,
}

impl ProtectedCrawler {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let browser = Self::create_browser()?;

        Ok(Self {
            browser: Arc::new(browser),
        })
    }

    fn create_browser() -> Result<Browser, Box<dyn Error>> {
        let fetcher_opts = FetcherOptions::default()
            .with_allow_download(true)
            .with_install_dir(Some("./chrome"));

        let launch_opts = LaunchOptionsBuilder::default()
            .fetcher_options(fetcher_opts)
            .build()?;

        let browser = Browser::new(launch_opts);

        Ok(browser?)
    }

    fn make_request(&self, url: &str) -> Result<Arc<Tab>, Box<dyn Error>> {
        let tab = self.browser.new_tab()?;
        tab.navigate_to(url)?;

        Ok(tab)
    }
}

impl Crawler for ProtectedCrawler {
    async fn make_web_request(&self, request: Request) -> Result<String, Box<dyn Error>> {
        let result = self.make_request(&request.url).unwrap().get_content();

        Ok(result?)
    }
}
