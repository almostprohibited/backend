use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::{request::Request, traits::HttpMethod};
use scraper::{ElementRef, Html, Selector};
use serde::{Deserialize, Serialize};
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        conversions::price_to_cents,
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const PAGE_SIZE: &str = "48";
const BASE_URL: &str = "https://www.reliablegun.com";
// so they have this in their robots.txt
// I've never been one to respect that file for any retailer
// but this is more friendly on their servers than normal pagination
const URL: &str = "https://www.reliablegun.com/catalog/es-filter";

#[derive(Serialize, Debug)]
#[serde(rename_all = "camelCase")]
struct ReliablePayload {
    order_by: String,
    page: String,
    page_size: String,
    view_mode: String,
    category_id: String,
}

impl ReliablePayload {
    fn new(category_id: String, page: u64) -> Self {
        Self {
            order_by: "0".into(),
            page: (page + 1).to_string(),
            page_size: PAGE_SIZE.into(),
            view_mode: "grid".into(),
            category_id,
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ReliableResponse {
    products_html: String,
    success: bool,
    total_pages: u64,
}

pub struct ReliableGun;

impl Default for ReliableGun {
    fn default() -> Self {
        Self::new()
    }
}

impl ReliableGun {
    pub fn new() -> Self {
        Self {}
    }

    fn find_prices(element: ElementRef) -> Result<Price, RetailerError> {
        let actual_element = extract_element_from_element(element, "span.actual-price")?;
        let actual_price = price_to_cents(element_to_text(actual_element))?;

        let mut price = Price {
            regular_price: actual_price,
            sale_price: None,
        };

        if let Ok(old_price_element) = extract_element_from_element(element, "span.old-price") {
            let old_price = price_to_cents(element_to_text(old_price_element))?;

            price.sale_price = Some(price.regular_price);
            price.regular_price = old_price;
        }

        Ok(price)
    }
}

impl HtmlRetailerSuper for ReliableGun {}

impl Retailer for ReliableGun {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::ReliableGun
    }
}

#[async_trait]
impl HtmlRetailer for ReliableGun {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let payload = ReliablePayload::new(search_term.term.clone(), page_num + 1);

        let request_builder = Request::builder()
            .set_url(URL)
            .set_method(HttpMethod::POST)
            .set_json_body(serde_json::to_value(payload)?)
            .set_headers([("Content-Type".into(), "application/json".into())].as_ref());

        Ok(request_builder.build())
    }

    async fn parse_response(
        &self,
        response: &String,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let deserialized_response = serde_json::from_str::<ReliableResponse>(response)?;

        if !deserialized_response.success {
            return Err(RetailerError::GeneralError(
                "Failed to perform Reliable call".into(),
            ));
        }

        let fragment = Html::parse_document(&deserialized_response.products_html);

        for element in fragment.select(&Selector::parse("div.product-item").unwrap()) {
            let description_element = extract_element_from_element(element, "div.description")?;
            let url_element = extract_element_from_element(element, "h2.product-title > a")?;
            let image_element = extract_element_from_element(element, "img.product-overview-img")?;

            let description = element_to_text(description_element);
            let name = element_to_text(url_element);

            let Ok(url_href) = element_extract_attr(url_element, "href") else {
                debug!("Skipping {name} as it contains no link (likely out of stock product)");
                continue;
            };

            let image_url = element_extract_attr(image_element, "src")?;

            let price = Self::find_prices(element)?;

            let formatted_name = match search_term.category {
                Category::Ammunition => format!("{} {}", name, description.clone()),
                _ => name,
            };

            let new_result = CrawlResult::new(
                formatted_name,
                format!("{BASE_URL}{url_href}"),
                price,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(image_url.to_string())
            .with_description(description);

            results.push(new_result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms = Vec::from_iter([
            HtmlSearchQuery {
                term: "1007".into(), // https://www.reliablegun.com/firearms
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "1002".into(), // https://www.reliablegun.com/ammunition
                category: Category::Ammunition,
            },
        ]);

        let other_terms = [
            "407",  // https://www.reliablegun.com/used-guns-non-restricted
            "680",  // https://www.reliablegun.com/used-optics
            "435",  // https://www.reliablegun.com/used-guns-restricted
            "1012", // https://www.reliablegun.com/optics
            "1013", // https://www.reliablegun.com/reloading
            "1014", // https://www.reliablegun.com/safes-and-cases
            "1015", // https://www.reliablegun.com/shooting-accessories
            "810",  // https://www.reliablegun.com/safety-glasses
            "1008", // https://www.reliablegun.com/gun-parts
            "1004", // https://www.reliablegun.com/books
            "1005", // https://www.reliablegun.com/cleaning-accessories
            "806",  // https://www.reliablegun.com/hearing-protection
            "602",  // https://www.reliablegun.com/flash-lights
            "610",  // https://www.reliablegun.com/tools
            "467",  // https://www.reliablegun.com/magazines
            "469",  // https://www.reliablegun.com/muzzle-brakes
        ];

        for other in other_terms {
            terms.push(HtmlSearchQuery {
                term: other.to_string(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let deserialized_response = serde_json::from_str::<ReliableResponse>(response)?;

        if !deserialized_response.success {
            return Err(RetailerError::GeneralError(
                "Failed to perform Reliable call".into(),
            ));
        }

        // I'm going to trust the resposne from Reliable
        Ok(deserialized_response.total_pages)
    }
}
