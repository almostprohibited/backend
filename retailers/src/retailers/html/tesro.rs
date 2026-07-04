// pretty sure tesro is a magento ecommerce backend
// but they seem to use some sort of theme

use std::time::Duration;

use async_trait::async_trait;
use common::{
    result::{
        base::{CrawlResult, Price},
        enums::{Category, RetailerName},
    },
    utils::get_api_call_delay,
};
use crawler::{
    WebClient,
    request::{Request, RequestBuilder},
};
use scraper::{ElementRef, Html, Selector};
use tokio::time::sleep;
use tracing::{debug, info, warn};

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        conversions::{price_to_cents, string_to_u64},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const URL: &str = "https://www.tesro.ca/{category}?limit=36&p={page}";

pub struct Tesro {}

impl Default for Tesro {
    fn default() -> Self {
        Self::new()
    }
}

impl Tesro {
    pub fn new() -> Self {
        Self {}
    }

    async fn parse_variant(
        &self,
        url: &str,
        category: Category,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = vec![];

        let request = RequestBuilder::new().set_url(url).build();
        let response = WebClient::make_web_request(request).await?;

        let html = Html::parse_document(&response.body);

        let image_element =
            extract_element_from_element(html.root_element(), "p.product-image img")?;
        let image_url = element_extract_attr(image_element, "src")?;

        let product_selector = Selector::parse("table#super-product-table > tbody > tr").unwrap();

        for element in html.select(&product_selector) {
            let price = Self::parse_price(element)?;

            // this relies on the first table cell being the name
            let name_element = extract_element_from_element(element, "td")?;

            results.push(
                CrawlResult::new(
                    element_to_text(name_element),
                    url.to_string(),
                    price,
                    self.get_retailer_name(),
                    category,
                )
                .with_image_url(image_url.clone()),
            );
        }

        Ok(results)
    }

    fn parse_price(element: ElementRef) -> Result<Price, RetailerError> {
        let price_wrapper = extract_element_from_element(element, "div.price-box")?;

        match extract_element_from_element(price_wrapper, "p.special-price > span.price") {
            Ok(sale_price) => {
                let normal_price =
                    extract_element_from_element(price_wrapper, "p.old-price > span.price")?;

                Ok(Price {
                    regular_price: price_to_cents(element_to_text(normal_price))?,
                    sale_price: Some(price_to_cents(element_to_text(sale_price))?),
                })
            }
            Err(_) => {
                let normal_price =
                    extract_element_from_element(price_wrapper, "span.regular-price > span.price")?;

                Ok(Price {
                    regular_price: price_to_cents(element_to_text(normal_price))?,
                    sale_price: None,
                })
            }
        }
    }
}

impl HtmlRetailerSuper for Tesro {}

impl Retailer for Tesro {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::Tesro
    }
}

#[async_trait]
impl HtmlRetailer for Tesro {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", (page_num + 1).to_string().as_str());

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

        let fragment = Html::parse_document(response);

        let product_selector = Selector::parse("ul.products-grid > li.item").unwrap();

        let mut variant_urls: Vec<String> = vec![];

        for element in fragment.select(&product_selector) {
            let Ok(action_button) =
                extract_element_from_element(element, "div.actions > button.btn-cart")
            else {
                info!("Skipping product that is not in stock");

                continue;
            };

            if element_to_text(action_button).to_lowercase() != "add to cart" {
                info!("Skipping preorder/backorder/special order product");

                continue;
            }

            let title_element = extract_element_from_element(element, "h2.product-name > a")?;

            let name = element_extract_attr(title_element, "title")?;
            let url = element_extract_attr(title_element, "href")?;

            let price_wrapper = extract_element_from_element(element, "div.price-box")?;

            if element_to_text(price_wrapper)
                .to_lowercase()
                .contains("starting at:")
            {
                info!("Will parse {} as variant", name);

                variant_urls.push(url);

                continue;
            }

            let image_element =
                extract_element_from_element(element, "div.product-image-wrapper > a > img")?;

            let image_url = element_extract_attr(image_element, "src")?;

            let result = CrawlResult::new(
                name,
                url,
                Self::parse_price(element)?,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(image_url);

            results.push(result);
        }

        for url in variant_urls {
            results.extend(self.parse_variant(&url, search_term.category).await?);

            sleep(Duration::from_secs(get_api_call_delay())).await;
        }

        Ok(results)
    }

    // hey have a sitemap but it's incomplete
    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms: Vec<HtmlSearchQuery> = vec![];

        for ammo in [
            "smallbore-ammunition",
            "pellets",
            "centerfire-ammunition",
            "pistol-ammunition",
        ] {
            terms.push(HtmlSearchQuery {
                term: format!("ammunition-and-pellets/{}.html", ammo.to_string()),
                category: Category::Ammunition,
            });
        }

        for firearm in [
            "smallbore-rifles-and-accessories",
            "centerfire-rifles",
            "custom-builds",
            "tesro-in-house-builds-centerfire",
            "hunting-rifles",
            "shotguns",
        ] {
            terms.push(HtmlSearchQuery {
                term: format!("rifles-and-pistols/{}.html", firearm.to_string()),
                category: Category::Firearm,
            });
        }

        for other in [
            "rifles-and-pistols/pistols", // yes this is pistols, but they have slides and kits in here
            "rifles-and-pistols/barreled-actions",
            "optics-sights",
            "rifle-and-pistol-accessories",
            "reloading/bullets",
            "reloading/powder",
            "reloading/primers",
            "reloading/brass",
            "reloading/lapua-hunting-bullets",
            "reloading/pistol-bullets",
            "equipment",
        ] {
            terms.push(HtmlSearchQuery {
                term: format!("{}.html", other.to_string()),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let fragment = Html::parse_document(response);

        if extract_element_from_element(fragment.root_element(), "p.out-of-stock").is_ok() {
            warn!("Page contains out of stock items, early exit");

            return Ok(0);
        }

        let page_number_selector =
            Selector::parse("div.pager > div > ol > li:not(.current):not(.previous):not(.next)")
                .unwrap();

        let mut page_links = fragment.select(&page_number_selector);

        let Some(last_page_element) = page_links.next_back() else {
            return Ok(0);
        };

        string_to_u64(element_to_text(last_page_element))
    }
}
