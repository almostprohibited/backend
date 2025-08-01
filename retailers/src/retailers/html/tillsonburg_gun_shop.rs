use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use regex::Regex;
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

const MAX_PER_PAGE: &str = "100";
const URL: &str = "https://tillsonburggunshop.com/{category}?limit={max_per_page}&page={page}";

pub struct Tillsonburg {
    page_count_regex: Regex,
}

impl Tillsonburg {
    pub fn new() -> Self {
        Self {
            page_count_regex: Regex::new(r"\((\d+)\s+Pages\)")
                .expect("Static regex should compile"),
        }
    }

    fn is_out_of_stock(element: ElementRef) -> Result<bool, RetailerError> {
        let query = "div > div.button-group > button > span.hidden-md";

        let Ok(cart_button) = extract_element_from_element(element, query) else {
            return Err(RetailerError::HtmlMissingElement(query.to_string()));
        };

        return Ok(element_to_text(cart_button) == "Out Of Stock");
    }

    fn get_price(element: ElementRef) -> Result<Price, RetailerError> {
        let mut price = Price {
            regular_price: 0,
            sale_price: None,
        };

        let price_element = extract_element_from_element(element, "p.price")?;

        match extract_element_from_element(price_element, "span.price-new") {
            Ok(sale_element) => {
                let regular_element =
                    extract_element_from_element(price_element, "span.price-old")?;

                price.regular_price = price_to_cents(element_to_text(regular_element))?;
                price.sale_price = Some(price_to_cents(element_to_text(sale_element))?);
            }
            Err(_) => {
                price.regular_price = price_to_cents(element_to_text(price_element))?;
            }
        }

        Ok(price)
    }

    fn clean_url(url: impl Into<String>) -> String {
        let converted_url: String = url.into();

        let Some((clean_url, _)) = converted_url.split_once("?") else {
            return converted_url;
        };

        clean_url.to_string()
    }
}

impl HtmlRetailerSuper for Tillsonburg {}

impl Retailer for Tillsonburg {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::Tillsonburg
    }
}

#[async_trait]
impl HtmlRetailer for Tillsonburg {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
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
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let html = Html::parse_document(&response);

        let product_selector =
            Selector::parse("div.row > div.product-layout > div.product-thumb").unwrap();

        for product in html.select(&product_selector) {
            if Self::is_out_of_stock(product)? {
                break;
            }

            let image_element = extract_element_from_element(product, "div.image > a > img")?;
            let image_url = element_extract_attr(image_element, "src")?;

            let title_element =
                extract_element_from_element(product, "div > div.caption > h4 > a")?;
            let name = element_to_text(title_element);
            let url = element_extract_attr(title_element, "href")?;
            let price = Self::get_price(product)?;

            let new_result = CrawlResult::new(
                name,
                Self::clean_url(url),
                price,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(Self::clean_url(image_url));

            results.push(new_result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms = Vec::from_iter([
            HtmlSearchQuery {
                term: "Firearms-guns-Rifles-Shotgun-pistol-handgun-Air-gun".into(),
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "Used-Firearms".into(),
                category: Category::Firearm,
            },
        ]);

        let other_terms = [
            "Optics-scopes-binoculars",
            "Reloading",
            "Firearm-Accessories",
            "shooting-accessories",
            "Gun-Cleaning-kit",
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
        let html = Html::parse_document(&response);
        let root = html.root_element();

        if Self::is_out_of_stock(root)? {
            return Ok(0);
        }

        let Ok(page_count) =
            extract_element_from_element(root, "div#content > div.row > div.text-right.col-sm-6")
        else {
            warn!("Missing page count text");
            return Ok(0);
        };

        let page_count_string = element_to_text(page_count);

        let Some(capture) = self.page_count_regex.captures(&page_count_string) else {
            warn!("Page string '{page_count_string}' did not match regex");
            return Ok(0);
        };

        let page_match = capture
            .get(1)
            .expect("Expect single match as there was a capture");

        Ok(string_to_u64(page_match.as_str().to_string())?)
    }
}
