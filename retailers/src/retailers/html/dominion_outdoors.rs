use std::time::Duration;

use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::{
    request::{Request, RequestBuilder},
    unprotected::UnprotectedCrawler,
};
use scraper::{ElementRef, Html, Selector};
use tokio::time::sleep;
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        conversions::{price_to_cents, string_to_u64},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const CRAWL_COOLDOWN_SECS: u64 = 3;
const URL: &str = "https://www.dominionoutdoors.ca/{category}/page{page}.html";

pub struct DominionOutdoors {}

impl Default for DominionOutdoors {
    fn default() -> Self {
        Self::new()
    }
}

impl DominionOutdoors {
    pub fn new() -> Self {
        Self {}
    }

    async fn send_request(&self, url: String) -> Result<String, RetailerError> {
        let request = RequestBuilder::new().set_url(url).build();
        let response = UnprotectedCrawler::make_web_request(request).await?;

        let response_body = response.body;

        Ok(response_body)
    }

    fn is_in_stock(element: ElementRef) -> bool {
        extract_element_from_element(element, "span.in-stock").is_ok()
    }

    async fn parse_page(
        &self,
        url: String,
        retailer: RetailerName,
        category: Category,
    ) -> Result<Option<CrawlResult>, RetailerError> {
        let page = self.send_request(url.clone()).await?;

        let html = Html::parse_document(&page);
        let page_element = html.root_element();

        if !Self::is_in_stock(page_element) {
            return Ok(None);
        }

        let title_element =
            extract_element_from_element(page_element, "h1.product-page[itemprop='name']")?;
        let name = element_to_text(title_element);

        let image_element = extract_element_from_element(page_element, "meta[itemprop='image']");
        let image = match image_element {
			Ok(element) => element_extract_attr(element, "content")?,
			Err(_) => "https://cdn.shoplightspeed.com/shops/644978/themes/13862/assets/pro-icon.png?20240402173321".into(),
		};

        let price_element =
            extract_element_from_element(page_element, "div.product-price > div > span.price")?;

        let mut price = element_to_text(price_element);
        if price.starts_with("C") {
            price = price.split_off(1);
        }

        // I don't know what the sale price looks like
        // YOLO
        let price = Price {
            regular_price: price_to_cents(price)?,
            sale_price: None,
        };

        let result = CrawlResult::new(name, url, price, retailer, category).with_image_url(image);

        Ok(Some(result))
    }
}

impl HtmlRetailerSuper for DominionOutdoors {}

impl Retailer for DominionOutdoors {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::DominionOutdoors
    }
}

#[async_trait]
impl HtmlRetailer for DominionOutdoors {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", &(page_num + 1).to_string());

        debug!("Setting page to {}", url);

        let request = RequestBuilder::new().set_url(url).build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let mut links: Vec<String> = Vec::new();

        // commit another Rust sin, and clone the entire HTML
        // as a string since scraper::ElementRef is not thread safe
        // we'll recreate the Node later
        let products = {
            let html = Html::parse_document(response);
            let product_selector =
                Selector::parse("div.products > div > div.product > div.info").unwrap();
            html.select(&product_selector)
                .map(|element| element.html().clone())
                .collect::<Vec<_>>()
        };

        for html_doc in products {
            let product_inner = Html::parse_fragment(&html_doc);
            let product = product_inner.root_element();

            let title_element = extract_element_from_element(product, "a")?;
            let url = element_extract_attr(title_element, "href")?;

            links.push(url);
        }

        for link in links {
            if let Some(result) = self
                .parse_page(link, self.get_retailer_name(), search_term.category)
                .await?
            {
                results.push(result);
            }

            sleep(Duration::from_secs(CRAWL_COOLDOWN_SECS)).await;
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms = Vec::from_iter([HtmlSearchQuery {
            term: "firearms".into(),
            category: Category::Firearm,
        }]);

        let other_terms = [
            "shooting",
            "optics",
            "muzzleloading-4305673",
            "reloading",
            "knives-tools/flashlights",
        ];

        for other in other_terms {
            terms.push(HtmlSearchQuery {
                term: other.into(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let fragment = Html::parse_document(response);
        let page_number_selector = Selector::parse("div.pager > ul.right > li.number").unwrap();

        let mut page_links = fragment.select(&page_number_selector);

        let Some(last_page_element) = page_links.next_back() else {
            return Ok(0);
        };

        string_to_u64(element_to_text(last_page_element))
    }
}
