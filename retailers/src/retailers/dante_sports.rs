use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{ElementRef, Html, Selector};
use tracing::debug;

use crate::{
    errors::RetailerError,
    traits::{Retailer, SearchTerm},
    utils::{
        ecommerce::woocommerce::WooCommerce,
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const MAX_PER_PAGE: &str = "48";
const URL: &str = "https://www.dantesports.com/en/product-category/{category}/page/{page}/?per_page={max_per_page}&availability=in-stock";

pub struct DanteSports {
    retailer: RetailerName,
}

impl DanteSports {
    pub fn new() -> Self {
        Self {
            retailer: RetailerName::DanteSports,
        }
    }

    // dante images are either under `data-src`, or `src` attributes
    // we should be using the former if it exists first
    fn get_image_url(image_element: ElementRef) -> Result<String, RetailerError> {
        if let Ok(data_src) = element_extract_attr(image_element, "data-src")
            && data_src.starts_with("https")
        {
            return Ok(data_src);
        };

        if let Ok(regular_src) = element_extract_attr(image_element, "src")
            && regular_src.starts_with("https")
        {
            return Ok(regular_src);
        }

        return Err(RetailerError::HtmlElementMissingAttribute(
            "'valid data-src or src'".into(),
            element_to_text(image_element),
        ));
    }
}

#[async_trait]
impl Retailer for DanteSports {
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
            .replace("{max_per_page", MAX_PER_PAGE);

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
            Selector::parse("ul#products > li.product > div > a.woocommerce-LoopProduct-link")
                .unwrap();

        for product in html.select(&product_selector) {
            let link = element_extract_attr(product, "href")?;

            let image_element =
                extract_element_from_element(product, "div.product-loop-thumbnail > img")?;

            let image_link = Self::get_image_url(image_element)?;

            let name_element =
                extract_element_from_element(product, "h2.woocommerce-loop-product__title")?;
            let name = element_to_text(name_element);

            let new_result = CrawlResult::new(
                name,
                link,
                WooCommerce::parse_price(product)?,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(image_link);

            results.push(new_result);
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
                term: "riflescopes-optics".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "reloading".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "storage".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories".into(),
                category: Category::Other,
            },
        ])
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        WooCommerce::parse_max_pages(response)
    }
}
