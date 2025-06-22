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
        conversions::{price_to_cents, string_to_u64},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const URL: &str = "https://leverarms.com/product-category/{catagory}/page/{page}/";

pub struct LeverArms {
    crawler: UnprotectedCrawler,
    retailer: RetailerName,
}

impl LeverArms {
    pub fn new() -> Self {
        Self {
            crawler: UnprotectedCrawler::new(),
            retailer: RetailerName::LeverArms,
        }
    }
}

#[async_trait]
impl Retailer for LeverArms {
    fn get_retailer_name(&self) -> RetailerName {
        self.retailer
    }

    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &SearchTerm,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{catagory}", &search_term.term)
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

        let product_selector = Selector::parse("a.woocommerce-LoopProduct-link").unwrap();

        for element in fragment.select(&product_selector) {
            let title_element =
                extract_element_from_element(element, "h2.woocommerce-loop-product__title")?;
            let price_element =
                extract_element_from_element(element, "span.woocommerce-Price-amount")?;
            let image_element =
                extract_element_from_element(element, "img.attachment-woocommerce_thumbnail");

            let link = element_extract_attr(element, "href")?;
            let title = element_to_text(title_element);
            let price = price_to_cents(element_to_text(price_element))?;

            // lever arms uses a place holder element for missing images
            let image_link = match image_element {
                Ok(unwrapped_img_el) => element_extract_attr(unwrapped_img_el, "src")?,
                Err(_) => {
                    "https://leverarms.com/wp-content/uploads/2021/07/placehold.jpg".to_string()
                }
            };

            if link.contains("/gunsmithing/") {
                continue;
            }

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
            .with_image_url(image_link.to_string());

            results.push(result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<SearchTerm> {
        Vec::from_iter([
            SearchTerm {
                term: "guns/rifles".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "guns/shotguns".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "guns/used".into(),
                category: Category::Firearm,
            },
            // don't bother parsing their other categories
            // they add products into more than one category
            // I'll parse out what I don't need later
            SearchTerm {
                term: "kit".into(),
                category: Category::Other,
            },
        ])
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let fragment = Html::parse_document(&response);
        let page_number_selector = Selector::parse("a.page-numbers").unwrap();

        let mut page_links = fragment.select(&page_number_selector);
        let page_links_count = page_links.clone().count();

        if page_links_count == 0 {
            // indicates no pages
            return Ok(0);
        }

        // page links look like:
        // ["1", "2", "3", "->"]
        // do `count - 2` to grab the number before the arrow
        let Some(last_page_element) = page_links.nth(page_links_count - 2) else {
            let message = format!("Invalid number of page elements: {:?}", page_links);
            error!(message);

            return Err(RetailerError::GeneralError(
                "Invalid number of page elements".into(),
            ));
        };

        Ok(string_to_u64(element_to_text(last_page_element))?)
    }

    fn get_crawler(&self) -> UnprotectedCrawler {
        self.crawler
    }

    fn get_page_cooldown(&self) -> u64 {
        1
    }
}
