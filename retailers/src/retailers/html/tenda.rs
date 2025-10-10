use async_trait::async_trait;
use base64::{Engine, prelude::BASE64_STANDARD};
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::{
    request::{Request, RequestBuilder},
    unprotected::UnprotectedCrawler,
};
use regex::Regex;
use scraper::{Html, Selector};
use tracing::{debug, trace};

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        auctollo_sitemap::get_search_queries,
        ecommerce::woocommerce::{WooCommerce, WooCommerceBuilder},
        regex::unwrap_regex_capture,
    },
};

const SITE_MAP: &str = "https://www.gotenda.com/product_cat-sitemap.xml";
const PRODUCT_BASE_URL: &str = "https://www.gotenda.com/product-category/";
const BASE_URL: &str = "https://www.gotenda.com/";
const URL: &str = "https://www.gotenda.com/product-category/{category}/page/{page}/?stock=instock";

pub struct Tenda {
    securi_cookie: String,
    search_terms: Vec<HtmlSearchQuery>,
}

impl Default for Tenda {
    fn default() -> Self {
        Self::new()
    }
}

impl Tenda {
    pub fn new() -> Self {
        Self {
            securi_cookie: String::new(),
            search_terms: Vec::new(),
        }
    }

    fn get_cookie_name(haystack: &str) -> Result<String, RetailerError> {
        let cookie_name_regex = Regex::new(r##";document\.cookie=(.*?)\+\s*\"=\"\s*\+"##)
            .expect("Regex should compile as nothing has changed");

        let cookie_name_obfuscated = unwrap_regex_capture(&cookie_name_regex, haystack)?;
        let mut cookie_name_parts: Vec<String> = Vec::new();

        for cooke_name_part in cookie_name_obfuscated.split("+") {
            let Some(individual_char) = cooke_name_part.get(1..2) else {
                return Err(RetailerError::GeneralError(format!(
                    "Failed to map value: {cooke_name_part}"
                )));
            };

            cookie_name_parts.push(individual_char.to_string());
        }

        Ok(cookie_name_parts.join(""))
    }

    fn get_cookie_value(haystack: &str) -> Result<String, RetailerError> {
        let obfuscated_string_regex =
            Regex::new(r"=(.*?)\s+\+\s+'';").expect("Regex should compile as nothing has changed");
        let char_code_regex = Regex::new(r"String\.fromCharCode\((\d+)\)")
            .expect("Regex should compile as nothing has changed");

        // the JS starts with `i=<string parts>;cookie`
        // I want the inside parts
        let cookie_value_obfuscated = unwrap_regex_capture(&obfuscated_string_regex, haystack)?;

        let mut reconstructed_parts: Vec<String> = Vec::new();

        let char_code_parts: Vec<&str> = cookie_value_obfuscated.split(" + ").collect();

        for part in char_code_parts {
            let Ok(char_code) = unwrap_regex_capture(&char_code_regex, part) else {
                let Some(individual_char) = part.get(1..2) else {
                    return Err(RetailerError::GeneralError(format!(
                        "Captured non String.fromCharCode, but failed to map to char: {part}"
                    )));
                };

                reconstructed_parts.push(individual_char.to_string());
                continue;
            };

            let Ok(char_code) = char_code.parse::<u32>() else {
                return Err(RetailerError::GeneralError(format!(
                    "Char code is not a number: {char_code}"
                )));
            };

            let Some(parsed_char) = char::from_u32(char_code) else {
                return Err(RetailerError::GeneralError(format!(
                    "Failed to convert char into valid UTF-8: {char_code}"
                )));
            };

            reconstructed_parts.push(parsed_char.to_string());
        }

        Ok(reconstructed_parts.join(""))
    }

    // SecURI's wordpress "firewall" might as well not be there
    // below is cursed Javascript to Rust translation code
    // (I don't want to explore Deno)
    async fn set_securi_cookie() -> Result<String, RetailerError> {
        let base64_regex = Regex::new(r"\bS\s*=\s*'([^']*)'")
            .expect("Regex should compile as nothing has changed");

        let crawler = UnprotectedCrawler::new();
        let request = RequestBuilder::new().set_url(BASE_URL).build();

        let result = crawler.make_web_request(request).await?;

        let base64 = unwrap_regex_capture(&base64_regex, &result.body)?;

        trace!("{base64}");

        let Ok(decoded_base64) = BASE64_STANDARD.decode(&base64) else {
            return Err(RetailerError::GeneralError(format!(
                "Failed to decode base64, got this instead: {base64}"
            )));
        };

        let Ok(decoded_string) = String::from_utf8(decoded_base64) else {
            return Err(RetailerError::GeneralError(
                "Invalid string, decoded base64 did not convert into a string".to_string(),
            ));
        };

        let cookie_name = Self::get_cookie_name(&decoded_string)?;
        let cookie_value = Self::get_cookie_value(&decoded_string)?;

        Ok(format!("{cookie_name}={cookie_value};"))
    }

    async fn get_search_queries() -> Result<Vec<HtmlSearchQuery>, RetailerError> {
        get_search_queries(SITE_MAP, PRODUCT_BASE_URL, |link| {
            if link.contains("/watches") || link.contains("/casual") || link.contains("/hats") {
                return None;
            }

            if link.starts_with("accessories/")
                || link.starts_with("reloading/")
                || link.starts_with("optic/")
            {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Other,
                });
            } else if link.starts_with("ammunition/") {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Ammunition,
                });
            } else if link.starts_with("firearms/") {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Firearm,
                });
            };

            None
        })
        .await
    }
}

impl HtmlRetailerSuper for Tenda {}

#[async_trait]
impl Retailer for Tenda {
    async fn init(&mut self) -> Result<(), RetailerError> {
        let cookie = Self::set_securi_cookie().await?;

        debug!("Using cookie: {cookie}");

        self.securi_cookie = cookie;
        self.search_terms.extend(Self::get_search_queries().await?);

        Ok(())
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::Tenda
    }
}

#[async_trait]
impl HtmlRetailer for Tenda {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", &(page_num + 1).to_string());

        debug!("Setting page to {}", url);

        let request = RequestBuilder::new()
            .set_url(url)
            .set_headers(&[("Cookie".into(), self.securi_cookie.clone())].to_vec())
            .build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let fragment = Html::parse_document(response);

        let product_selector = Selector::parse("ul.products > li.product.instock").unwrap();

        let woocommerce_helper = WooCommerceBuilder::default()
            .with_product_url_selector("h3.products-title > a")
            .with_product_name_selector("h3.products-title > a")
            .with_image_url_selector("figure.products-img > a > img")
            .build();

        for element in fragment.select(&product_selector) {
            results.push(woocommerce_helper.parse_product(
                element,
                self.get_retailer_name(),
                search_term.category,
            )?);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        // sucks to clone this, but I don't remember if this is run in a loop
        self.search_terms.clone()
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        WooCommerce::parse_max_pages(response)
    }
}
