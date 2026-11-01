// https://googlechromelabs.github.io/chrome-for-testing/
// https://storage.googleapis.com/chrome-for-testing-public/154.0.8037.92/linux64/chrome-linux64.zip
// https://storage.googleapis.com/chrome-for-testing-public/154.0.8037.92/linux-arm64/chrome-linux-arm64.zip

use std::{env, time::Duration};

use async_trait::async_trait;
use chromiumoxide::{
    Browser, Page,
    cdp::browser_protocol::network::{
        CookieParamBuilder, EventResponseReceived, Headers, ResourceType,
    },
};
use common::{constants::CHROME_URL, string_utils::get_domain};
use futures::StreamExt;
use reqwest::{StatusCode, header::HeaderMap};
use tokio::{
    sync::{RwLock, RwLockReadGuard},
    task::JoinHandle,
    time::timeout,
};
use tracing::{debug, warn};

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

async fn initialize_browser() -> Result<BrowserProps, CrawlerError> {
    debug!("Starting browser");

    let (browser, mut cdp_handler) = Browser::connect(
        env::var(format!("{CHROME_URL}")).expect(&format!("Missing {CHROME_URL}")),
    )
    .await?;

    let handler_thread = tokio::spawn(async move {
        debug!("Starting poller");

        while let Some(command) = cdp_handler.next().await {
            if command.is_err() {
                break;
            }
        }
    });

    Ok(BrowserProps {
        browser,
        cdp_handler: handler_thread,
    })
}

async fn get_browser() -> Result<RwLockReadGuard<'static, Browser>, CrawlerError> {
    loop {
        if let Ok(browser) = RwLockReadGuard::try_map(BROWSER.read().await, |result| {
            result.as_ref().map(|props| &props.browser)
        }) {
            return Ok(browser);
        }

        let mut write_lock = BROWSER.write().await;

        if write_lock.is_none() {
            *write_lock = Some(initialize_browser().await?);
        }
    }
}

pub(crate) struct BrowserClient {}

#[async_trait]
impl Client for BrowserClient {
    async fn set_cookie(&self, url: &str, cookie: &str) -> Result<(), CrawlerError> {
        let cookie = CookieParamBuilder::default()
            .domain(url)
            .value(cookie)
            .build()
            .unwrap();

        let _ = get_browser().await?.set_cookies(vec![cookie]).await;

        Ok(())
    }

    async fn send_request(&self, request: Request) -> Result<CrawlerResponse, CrawlerError> {
        debug!("Create tab for {}", request.url);

        let page = get_browser().await?.new_page("about:blank").await?;

        let result = self.send_request_inner(&page, &request).await;

        debug!("Kill tab: {}", request.url);
        if let Err(err) = page.close().await {
            warn!("Failed to close tab: {}", err);
        }

        result
    }
}

impl BrowserClient {
    async fn send_request_inner(
        &self,
        page: &Page,
        request: &Request,
    ) -> Result<CrawlerResponse, CrawlerError> {
        debug!("Register listener for {}", request.url);

        let document_watcher = self.register_page_watcher(&page, &request.url).await?;

        debug!("Sending browser request for {}", request.url);
        page.goto(request.url.clone()).await?;

        debug!("Waiting for response {}", request.url);
        // TODO: map browser headers to our own headers in response
        // response currently defaults to empty map
        let response_metadata = document_watcher
            .await
            .map_err(|err| CrawlerError::UnhandledError(err.to_string()))?
            .ok_or(CrawlerError::BrowserMissingHeaders)?
            .inner()
            .clone();

        let response_nav = page
            .wait_for_navigation_response()
            .await?
            .ok_or(CrawlerError::BrowserMissingResponse)?;

        let status_code = response_nav
            .response
            .clone()
            .ok_or(CrawlerError::BrowserMissingStatus)?
            .status as u16;

        debug!("{:?}", response_metadata);

        let response = page.content().await?;
        let response_bytes = page.content_bytes().await?;

        Ok(CrawlerResponse {
            body: response,
            raw_bytes: response_bytes.to_vec(),
            response_code: StatusCode::from_u16(status_code)
                .map_err(|_| CrawlerError::BrowserInvalidStatus(status_code.to_string()))?,
            headers: HeaderMap::default(),
        })
    }

    pub async fn kill_browser() {
        let Some(props) = BROWSER.write().await.take() else {
            return;
        };

        props.cdp_handler.abort();
    }

    async fn register_page_watcher(
        &self,
        page: &Page,
        url: &str,
    ) -> Result<JoinHandle<Option<Headers>>, CrawlerError> {
        let domain = get_domain(url);
        let url = url.to_string();

        let mut events = page.event_listener::<EventResponseReceived>().await?;

        let task = tokio::spawn(async move {
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
        });

        Ok(task)
    }
}
