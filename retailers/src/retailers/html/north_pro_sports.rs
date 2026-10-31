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
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        conversions::{price_to_cents, string_to_u64},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const URL: &str =
    "https://northprosports.com/index.php?route=product/category&path={category}&page={page}";

pub struct NorthProSports;

impl Default for NorthProSports {
    fn default() -> Self {
        Self::new()
    }
}

impl NorthProSports {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for NorthProSports {}

impl Retailer for NorthProSports {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::NorthProSports
    }
}

impl NorthProSports {
    fn get_price(element: ElementRef) -> Result<Price, RetailerError> {
        match extract_element_from_element(element, "span.price-old") {
            Ok(old_price) => {
                let sale_price = extract_element_from_element(element, "span.price-new")?;

                Ok(Price {
                    regular_price: price_to_cents(element_to_text(old_price))?,
                    sale_price: Some(price_to_cents(element_to_text(sale_price))?),
                })
            }
            Err(_) => {
                let caption_text = element_to_text(element);
                let caption_text = caption_text.trim();

                let Some((_, price)) = caption_text.split_once("$") else {
                    return Err(RetailerError::HtmlMissingElement(format!(
                        "Missing valid price, got: {caption_text}"
                    )));
                };

                Ok(Price {
                    regular_price: price_to_cents(price.trim().to_owned())?,
                    sale_price: None,
                })
            }
        }
    }
}

#[async_trait]
impl HtmlRetailer for NorthProSports {
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

        let product_selector = Selector::parse("div.row > div.product-layout").unwrap();

        for element in fragment.select(&product_selector) {
            let caption = extract_element_from_element(element, "div.caption > h4 > a")?;

            // ah yes, the out of stock item that is listed in the "in stock" category
            if element_to_text(caption).to_lowercase().contains("sold out") {
                warn!("Skipping out of stock product");

                continue;
            }

            let price = Self::get_price(caption)?;

            let image_ahref = extract_element_from_element(element, "div.image > a")?;
            let image_element = extract_element_from_element(image_ahref, "img")?;

            let url = element_extract_attr(image_ahref, "href")?;
            let name = element_extract_attr(image_element, "title")?;
            let image_url = element_extract_attr(image_element, "src")?;

            let result = CrawlResult::new(
                name,
                url,
                price,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(image_url);

            results.push(result);
        }

        Ok(results)
    }

    // hate it, random numbers are even more brittle than strings
    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        vec![
            HtmlSearchQuery {
                term: "45".into(), // ARCHERY
                category: Category::Other,
            },
            HtmlSearchQuery {
                term: "766".into(), // AMMUNITION
                category: Category::Ammunition,
            },
            HtmlSearchQuery {
                term: "28".into(), // FIREARMS
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "400".into(), // HUNTING ACCESSORIES
                category: Category::Other,
            },
            HtmlSearchQuery {
                term: "552".into(), // OPTICS
                category: Category::Other,
            },
            HtmlSearchQuery {
                term: "32".into(), // CLOTHING & FOOTWEAR
                category: Category::Other,
            },
            HtmlSearchQuery {
                term: "267".into(), // RELOADING
                category: Category::Other,
            },
        ]
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let fragment = Html::parse_document(response);

        let page_number_selector = Selector::parse("ul.pagination > li > a[data-page]").unwrap();

        let mut page_links = fragment.select(&page_number_selector);

        let Some(last_page_element) = page_links.next_back() else {
            return Ok(0);
        };

        string_to_u64(element_extract_attr(last_page_element, "data-page")?)
    }
}
