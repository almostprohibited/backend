use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::{
    request::{Request, RequestBuilder},
    unprotected::UnprotectedCrawler,
};
use scraper::{Html, Selector};
use tracing::{debug, error};

use crate::{
    errors::RetailerError,
    traits::{Retailer, SearchTerm},
    utils::{
        conversions::price_to_cents,
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

// items per page is constant, for some reason
const ITEM_PER_PAGE: u64 = 255;
const PAGE_COOLDOWN: u64 = 10;
const URL: &str = "https://www.canadasgunstore.ca/departments/{category}.html?top={count}";

pub struct CanadasGunStore {
    crawler: UnprotectedCrawler,
    retailer: RetailerName,
}

impl CanadasGunStore {
    pub fn new() -> Self {
        Self {
            crawler: UnprotectedCrawler::new(),
            retailer: RetailerName::CanadasGunStore,
        }
    }
}

#[async_trait]
impl Retailer for CanadasGunStore {
    fn get_retailer_name(&self) -> RetailerName {
        self.retailer
    }

    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &SearchTerm,
    ) -> Result<Request, RetailerError> {
        let request = RequestBuilder::new()
            .set_url(
                URL.replace("{category}", &search_term.term)
                    .replace("{count}", &(page_num * ITEM_PER_PAGE).to_string()),
            )
            .build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_term: &SearchTerm,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let html = Html::parse_document(response);

        let product_selector = Selector::parse("div.product_body").unwrap();

        for product in html.select(&product_selector) {
            let stock_element = extract_element_from_element(product, "span.product_status")?;

            if element_to_text(stock_element) != "In Stock" {
                debug!("Skipping out of stock item");
                continue;
            }

            let name_link_element =
                extract_element_from_element(product, "h4.store_product_name > a")?;
            let image_element = extract_element_from_element(product, "img.product_image")?;
            let price_element = extract_element_from_element(product, "div.product_price")?;
            let url = format!(
                "https://www.canadasgunstore.ca{}",
                element_extract_attr(name_link_element, "href")?
            );

            let name = element_to_text(name_link_element);
            let image = element_extract_attr(image_element, "src")?;

            let price = price_to_cents(element_to_text(price_element))?;

            let firearm_price = Price {
                regular_price: price,
                sale_price: None,
            };

            let new_result = CrawlResult::new(
                name,
                url,
                firearm_price,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(image.to_string());

            results.push(new_result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<SearchTerm> {
        Vec::from_iter([
            SearchTerm {
                term: "firearms-%7C30%7CFA".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "optics-%7C30%7COPT".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "shooting-%7C30%7CSHO".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "optics-%7C30%7COPT".into(),
                category: Category::Other,
            },
        ])
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(response);

        let Ok(page_count_element) = extract_element_from_element(
            html.root_element(),
            "div.store_results_navigation_top_wrapper > p.text-success",
        ) else {
            return Ok(0);
        };

        // 694 found, showing page 1 of 3
        let results_text = element_to_text(page_count_element);
        let results_parts: Vec<&str> = results_text.split(" ").collect();

        let Some(pages) = results_parts.last() else {
            let message = format!(
                "Failed to split the string (empty matches): {}",
                results_text
            );

            error!(message);

            return Err(RetailerError::GeneralError(message));
        };

        let Ok(page_as_num) = pages.parse::<u64>() else {
            error!(
                "{}",
                format!("Failed to convert string into number: {}", pages)
            );

            return Err(RetailerError::InvalidNumber(pages.to_string()));
        };

        Ok(page_as_num)
    }

    fn get_crawler(&self) -> UnprotectedCrawler {
        self.crawler
    }

    fn get_page_cooldown(&self) -> u64 {
        PAGE_COOLDOWN
    }
}
