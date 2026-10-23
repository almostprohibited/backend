// https://googlechromelabs.github.io/chrome-for-testing/
// https://storage.googleapis.com/chrome-for-testing-public/154.0.8037.92/linux64/chrome-linux64.zip

use std::{env, time::Duration};

use async_trait::async_trait;
use chromiumoxide::{
    Browser, BrowserConfig, Page,
    cdp::browser_protocol::network::{
        CookieParamBuilder, EventResponseReceived, Headers, ResourceType,
    },
};
use common::{constants::CHROME_PATH, string_utils::get_domain};
use futures::StreamExt;
use reqwest::{StatusCode, header::HeaderMap};
use tokio::{
    sync::{RwLock, RwLockReadGuard},
    task::JoinHandle,
    time::timeout,
};
use tracing::debug;

use crate::{
    constants::PAGE_TIMEOUT_SECONDS,
    errors::CrawlerError,
    request::Request,
    traits::{Client, CrawlerResponse},
};

struct BrowserProps {
    browser: Browser,
    cdp_handler: JoinHandle<()>,
}

static BROWSER: RwLock<Option<BrowserProps>> = RwLock::const_new(None);

async fn initialize_browser() -> BrowserProps {
    debug!("Starting browser");

    let (browser, mut cdp_handler) = Browser::launch(
        BrowserConfig::builder()
            .chrome_executable(
                env::var(format!("{CHROME_PATH}")).expect(&format!("Missing {CHROME_PATH}")),
            )
            .with_head()
            .hide()
            .window_size(1680, 1050)
            .viewport(None)
            .disable_default_args()
            .arg("no-first-run")
            .arg(("lang", "en_US"))
            .build()
            .unwrap(),
    )
    .await
    .unwrap();

    let handler_thread = tokio::spawn(async move {
        debug!("Starting poller");

        while let Some(command) = cdp_handler.next().await {
            if command.is_err() {
                break;
            }
        }
    });

    BrowserProps {
        browser,
        cdp_handler: handler_thread,
    }
}

async fn get_browser() -> RwLockReadGuard<'static, Browser> {
    loop {
        if let Ok(browser) = RwLockReadGuard::try_map(BROWSER.read().await, |result| {
            result.as_ref().map(|props| &props.browser)
        }) {
            return browser;
        }

        let mut write_lock = BROWSER.write().await;

        if write_lock.is_none() {
            *write_lock = Some(initialize_browser().await);
        }
    }
}

pub(crate) struct BrowserClient {}

#[async_trait]
impl Client for BrowserClient {
    async fn set_cookie(&self, url: &str, cookie: &str) {
        let cookie = CookieParamBuilder::default()
            .domain(url)
            .value(cookie)
            .build()
            .unwrap();

        let _ = get_browser().await.set_cookies(vec![cookie]).await;
    }

    async fn send_request(&self, request: Request) -> Result<CrawlerResponse, CrawlerError> {
        debug!("Create tab for {}", request.url);

        let page = get_browser().await.new_page("about:blank").await.unwrap();

        debug!("Register listener for {}", request.url);
        let document_watcher = self.register_page_watcher(&page, &request.url).await;

        debug!("Sending browser request for {}", request.url);
        page.goto(request.url.clone()).await.unwrap();

        debug!("Waiting for response {}", request.url);
        let response_metadata = document_watcher.await.unwrap().unwrap().inner().clone();

        let response_nav = page.wait_for_navigation_response().await.unwrap().unwrap();
        let status_code = response_nav.response.clone().unwrap().status as u16;

        debug!("{:?}", response_metadata);

        let response = page.content().await.unwrap();
        let response_bytes = page.content_bytes().await.unwrap();

        debug!("Kill tab: {}", request.url);
        let _ = page.close().await;

        Ok(CrawlerResponse {
            body: response,
            raw_bytes: response_bytes.to_vec(),
            response_code: StatusCode::from_u16(status_code).unwrap(),
            headers: HeaderMap::default(),
        })
    }
}

impl BrowserClient {
    pub async fn kill_browser() {
        let Some(mut props) = BROWSER.write().await.take() else {
            return;
        };

        // "This method is provided to help if the browser does not close by itself. You should prefer to use Browser::close."
        // I will proceed to ignore this advice, thanks
        props.browser.kill().await;

        let _ = props.cdp_handler.await;
    }

    async fn register_page_watcher(&self, page: &Page, url: &str) -> JoinHandle<Option<Headers>> {
        let domain = get_domain(url);
        let url = url.to_string();

        let mut events = page
            .event_listener::<EventResponseReceived>()
            .await
            .unwrap();

        tokio::spawn(async move {
            debug!("Starting task spawn {url}");

            let result = timeout(Duration::from_secs(PAGE_TIMEOUT_SECONDS), async move {
                while let Some(event) = events.next().await {
                    let conditions: Vec<bool> = vec![
                        event.r#type == ResourceType::Document,
                        event.response.status >= 200,
                        event.response.status < 300,
                        event.response.url.contains(&domain),
                        event
                            .response
                            .headers
                            .inner()
                            .get("content-type")
                            .unwrap_or_default()
                            .to_string()
                            .contains("text/html"),
                    ];

                    if event.r#type == ResourceType::Document {
                        debug!(
                            "[{:?}]: {}\n{} {:?}",
                            event.r#type,
                            event.response.url,
                            event.response.status,
                            event.response.headers.inner()
                        );
                    }

                    if conditions.iter().all(|cond| *cond) {
                        debug!("{url} loaded");
                        return Some(event.response.headers.clone());
                    }
                }

                debug!("{url} timed out");

                None
            })
            .await;

            result.unwrap_or_default()
        })
    }
}
