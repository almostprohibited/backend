use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{ElementRef, Html, Selector};
use tracing::{debug, warn};

use crate::{
    errors::RetailerError,
    traits::{Retailer, SearchTerm},
    utils::{
        conversions::{price_to_cents, string_to_u64},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const MAX_ITEMS_PER_PAGE: u64 = 24;
const URL: &str = "https://rdsc.ca/{category}.html?p={page}";

pub struct Rdsc {
    retailer: RetailerName,
}

impl Rdsc {
    pub fn new() -> Self {
        Self {
            retailer: RetailerName::Rdsc,
        }
    }

    fn is_in_stock(element: ElementRef) -> bool {
        // we have "ADD TO CART" button
        if extract_element_from_element(element, "button.action.tocart.primary").is_ok() {
            return true;
        }

        // check for button text to make sure it says "View Options"
        match extract_element_from_element(element, "a.view-button") {
            Ok(view_button) => {
                return element_to_text(view_button) == "View Options";
            }
            Err(_) => {
                return false;
            }
        }
    }
}

#[async_trait]
impl Retailer for Rdsc {
    fn get_retailer_name(&self) -> RetailerName {
        self.retailer
    }

    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &SearchTerm,
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
        search_term: &SearchTerm,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let fragment = Html::parse_document(&response);

        let product_selector =
            Selector::parse("ol.product-items > li.product-item > div.product-item-info").unwrap();

        for element in fragment.select(&product_selector) {
            if !Self::is_in_stock(element) {
                continue;
            }

            let title_element = extract_element_from_element(element, "a.product-item-link")?;
            let link = element_extract_attr(title_element, "href")?;
            let title = element_to_text(title_element);

            let image_element = extract_element_from_element(element, "img.product-image-photo")?;
            let image_url = element_extract_attr(image_element, "src")?;

            let price_element = extract_element_from_element(element, "span.price-wrapper")?;
            let price = element_extract_attr(price_element, "data-price-amount")?;

            let price = price_to_cents(price)?;

            let result = CrawlResult::new(
                title,
                link,
                Price {
                    regular_price: price,
                    sale_price: None,
                },
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(image_url.to_string());

            results.push(result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<SearchTerm> {
        let mut terms = Vec::from_iter([]);

        let firearm_terms = [
            "firearms-ammunition/pump-shotguns",
            "firearms-ammunition/semi-auto-rifles",
            "firearms-ammunition/bolt-action-rifles",
            "firearms-ammunition/revolvers",
            "firearms-ammunition/semi-auto-shotguns",
            "firearms-ammunition/lever-action-rifles",
            // "firearms-ammunition/semi-auto-handguns",
        ];

        for firearm in firearm_terms {
            terms.push(SearchTerm {
                term: firearm.into(),
                category: Category::Firearm,
            });
        }

        let other_terms = [
            "optics-mounts",
            "handgun-parts",
            "semi-auto-rifle-parts",
            "precision-rifle-parts",
            "lever-action-parts",
            "shotgun-parts",
            "gear-kit",
            "clean-maintain",
        ];

        for other in other_terms {
            terms.push(SearchTerm {
                term: other.into(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let fragment = Html::parse_document(&response);

        // there's no way to only filter for in stock items via the website
        // need to check if any of the item cart buttons exists, otherwise its
        // a dud page
        if !Self::is_in_stock(fragment.root_element()) {
            warn!("Page contains not in stock items");
            return Ok(0);
        }

        let item_number_selector =
            Selector::parse("p#toolbar-amount > span.toolbar-number").unwrap();
        let Some(item_number_element) = fragment.select(&item_number_selector).last() else {
            warn!("Page is missing total item count");
            return Ok(0);
        };

        let item_number_string = element_to_text(item_number_element);
        let total_count_num = string_to_u64(item_number_string)? as f32;

        Ok((total_count_num / MAX_ITEMS_PER_PAGE as f32).ceil() as u64)
    }
}
