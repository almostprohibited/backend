use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{ElementRef, Html, Selector};
use tracing::debug;

use crate::{
    errors::RetailerError,
    traits::{Retailer, SearchTerm},
    utils::{
        conversions::{price_to_cents, string_to_u64},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const MAX_PER_PAGE: &str = "48";
const URL: &str =
    "https://g4cgunstore.com/product-category/{category}/page/{page}/?per_page={max_per_page}";

pub struct G4CGunStore {
    retailer: RetailerName,
}

impl G4CGunStore {
    pub fn new() -> Self {
        Self {
            retailer: RetailerName::G4CGunStore,
        }
    }

    fn is_in_stock(element: ElementRef) -> bool {
        return extract_element_from_element(element, "div.product-element-bottom > div.in-stock")
            .is_ok();
    }

    fn is_dead_page(element: ElementRef) -> bool {
        return extract_element_from_element(
            element,
            "div.product-element-bottom > div.out-of-stock",
        )
        .is_ok();
    }

    fn get_price(element: ElementRef) -> Result<Price, RetailerError> {
        let mut price = Price {
            regular_price: 0,
            sale_price: None,
        };

        let regular_non_sale_price =
            extract_element_from_element(element, "span.price > span.amount > bdi");

        match regular_non_sale_price {
            Ok(regular_price_element) => {
                price.regular_price = price_to_cents(element_to_text(regular_price_element))?;
            }
            Err(_) => {
                let sale_price =
                    extract_element_from_element(element, "span.price > ins > span.amount > bdi")?;
                let previous_price =
                    extract_element_from_element(element, "span.price > del > span.amount > bdi")?;

                price.regular_price = price_to_cents(element_to_text(previous_price))?;
                price.sale_price = Some(price_to_cents(element_to_text(sale_price))?);
            }
        }

        Ok(price)
    }
}

#[async_trait]
impl Retailer for G4CGunStore {
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
            .replace("{page}", &(page_num + 1).to_string())
            .replace("{max_per_page}", MAX_PER_PAGE);

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

        let html = Html::parse_document(&response);

        let product_selector =
            Selector::parse("div.products > div.product > div.product-wrapper").unwrap();

        for product in html.select(&product_selector) {
            if !Self::is_in_stock(product) {
                // break instead of continue since products are in order
                // of in stock first, then all out of stock after
                break;
            }

            let image_element =
                extract_element_from_element(product, "a.product-image-link > img")?;
            let image_url = element_extract_attr(image_element, "src")?;

            let title_element =
                extract_element_from_element(product, "div.product-element-bottom > h3 > a")?;
            let name = element_to_text(title_element);
            let url = element_extract_attr(title_element, "href")?;

            let new_product = CrawlResult::new(
                name,
                url,
                Self::get_price(product)?,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(image_url);

            results.push(new_product);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<SearchTerm> {
        Vec::from_iter([
            SearchTerm {
                term: "firearms".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "sights-optics".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories".into(),
                category: Category::Other,
            },
        ])
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(&response);
        let root_element = html.root_element();

        if Self::is_dead_page(root_element) {
            return Ok(0);
        }

        let pagination_selector =
            Selector::parse("ul.page-numbers > li > a:not(.next):not(.prev).page-numbers").unwrap();

        let Some(last_page) = root_element.select(&pagination_selector).last() else {
            return Ok(0);
        };

        Ok(string_to_u64(element_to_text(last_page))?)
    }
}
