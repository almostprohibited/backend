use std::time::Instant;

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
use scraper::{ElementRef, Html, Selector};
use sha1::{Digest, Sha1};
use tracing::{debug, info};

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        ecommerce::{WooCommerce, WooCommerceBuilder},
        html::{element_extract_attr, extract_element_from_element},
        regex::unwrap_regex_capture,
    },
};

const MAX_PER_PAGE: &str = "48";
const BASE_URL: &str = "https://g4cgunstore.com";
const URL: &str =
    "https://g4cgunstore.com/product-category/{category}/page/{page}/?per_page={max_per_page}";

#[derive(Debug)]
struct ChallengeSolution {
    matching_hash: String,
    elapsed_time: u64,
    counter: u32,
}

impl ChallengeSolution {
    fn get_submit_url(&self) -> String {
        format!(
            "https://g4cgunstore.com/.well-known/sgcaptcha/?r=%2F&sol={}%3D%3D&s={}:{}",
            self.matching_hash, self.elapsed_time, self.counter
        )
    }
}

pub struct G4CGunStore;

impl Default for G4CGunStore {
    fn default() -> Self {
        Self::new()
    }
}

impl G4CGunStore {
    pub fn new() -> Self {
        Self {}
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

    fn extract_sg_redirect(html: &str) -> Result<String, RetailerError> {
        // this is just a redirct to the well-known URI
        // technically not needed since all I need is the IP
        // but they provide it, so why not

        let html = Html::parse_document(html);
        let meta = extract_element_from_element(html.root_element(), "meta")?;
        let content = element_extract_attr(meta, "content")?;

        let Some((_, uri)) = content.split_once(";") else {
            return Err(RetailerError::GeneralError(
                "Invalid meta; expected content to contain semicolon".to_string(),
            ));
        };

        Ok(format!("{BASE_URL}{uri}"))
    }

    async fn get_captcha_challenge(url: &str) -> Result<String, RetailerError> {
        let regex =
            Regex::new(r#"const\s+sgchallenge\s*=\s*"([^"]+)""#).expect("Static regex to compile");

        let request = RequestBuilder::new().set_url(url).build();
        let result = UnprotectedCrawler::make_web_request(request).await?;

        let challenge = unwrap_regex_capture(&regex, &result.body)?;

        Ok(challenge)
    }

    fn get_difficulty(challenge_string: &str) -> Result<u32, RetailerError> {
        let Some((difficulty, _)) = challenge_string.split_once(":") else {
            return Err(RetailerError::GeneralError(format!(
                "Expected challenge string to contain colon, instead got: \"{challenge_string}\""
            )));
        };

        let Ok(difficulty_int) = difficulty.parse::<u32>() else {
            return Err(RetailerError::GeneralError(format!(
                "Invalid difficulty, expected int, got: \"{difficulty}\""
            )));
        };

        if difficulty_int > 32 {
            return Err(RetailerError::GeneralError(format!(
                "Invalid difficulty, expected int < 32, got: \"{difficulty_int}\""
            )));
        }

        Ok(difficulty_int)
    }

    fn convert_counter(counter: u32) -> Vec<u8> {
        let word = counter.to_be_bytes();

        // u32 will always result in 4 byte array
        // meaning defaulting to `3` will pull `0`
        // in the case of first counter
        let position = word.iter().position(|&byte| byte != 0).unwrap_or(3);

        word[position..].to_vec()
    }

    fn validate_hash(hash: &[u8; 20], difficulty: u32) -> bool {
        // this assumes that Site Ground doesn't do something dumb
        // and set the difficulty to something more than 32 bits
        let first_32_bits = u32::from_be_bytes(
            hash[..4]
                .try_into()
                .expect("SHA1 result to always be 20 bytes"),
        );

        first_32_bits.leading_zeros() >= difficulty
    }

    fn solve_challenge(challenge_string: &str) -> Result<ChallengeSolution, RetailerError> {
        let start_time = Instant::now();

        let difficulty = Self::get_difficulty(challenge_string)?;
        let nonce_bytes = challenge_string.as_bytes();

        let mut counter: u32 = 0;

        let mut hasher = Sha1::new();
        let mut pow_input: Vec<u8> = Vec::new();
        pow_input.extend(nonce_bytes);

        info!("Begin proof of work");

        // https://www.desmos.com/calculator/mitivszs7g
        // 99% confidence
        while counter < 4828869 {
            let big_endian_counter = Self::convert_counter(counter);

            // delete counter and re-append
            pow_input.truncate(nonce_bytes.len());
            pow_input.extend(big_endian_counter);

            hasher.update(&pow_input);

            let hash: [u8; 20] = hasher.finalize_reset().into();

            counter += 1;

            if Self::validate_hash(&hash, difficulty) {
                return Ok(ChallengeSolution {
                    matching_hash: BASE64_STANDARD.encode(&pow_input),
                    // as_millis() returns u128, I really hope that
                    // I don't need 16 full bytes for this
                    elapsed_time: start_time.elapsed().as_millis() as u64,
                    counter: counter,
                });
            }
        }

        Err(RetailerError::GeneralError(
            "Failed to find solution".to_string(),
        ))
    }
}

impl HtmlRetailerSuper for G4CGunStore {}

#[async_trait]
impl Retailer for G4CGunStore {
    async fn init(&mut self) -> Result<(), RetailerError> {
        let request = RequestBuilder::new().set_url(BASE_URL).build();
        let result = UnprotectedCrawler::make_web_request(request).await?;

        if result.body.contains("/.well-known/sgcaptcha/") {
            // siteground really expects a client sided check to fix bots
            // from visiting the site, okay

            info!("Solving Site Ground challenge");

            let redirect = Self::extract_sg_redirect(&result.body)?;
            let challenge = Self::get_captcha_challenge(&redirect).await?;

            info!("Extracted challenge string: \"{challenge}\"");

            let solution = Self::solve_challenge(&challenge)?;
            let request = RequestBuilder::new()
                .set_url(solution.get_submit_url())
                .build();
            let _ = UnprotectedCrawler::make_web_request(request).await?;
        }

        Ok(())
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::G4CGunStore
    }
}

#[async_trait]
impl HtmlRetailer for G4CGunStore {
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

        let html = Html::parse_document(response);

        let product_selector =
            Selector::parse("div.products > div.product > div.product-wrapper").unwrap();

        let woocommerce_helper = WooCommerceBuilder::default().build();

        for product in html.select(&product_selector) {
            if !Self::is_in_stock(product) {
                // break instead of continue since products are in order
                // of in stock first, then all out of stock after
                break;
            }

            let result = woocommerce_helper.parse_product(
                product,
                self.get_retailer_name(),
                search_term.category,
            )?;

            results.push(result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms = Vec::from_iter([
            HtmlSearchQuery {
                term: "firearms".into(),
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "Ammunition".into(),
                category: Category::Ammunition,
            },
        ]);

        let other_terms = ["sights-optics", "accessories"];

        for other in other_terms {
            terms.push(HtmlSearchQuery {
                term: other.into(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(response);
        let root_element = html.root_element();

        if Self::is_dead_page(root_element) {
            return Ok(0);
        }

        WooCommerce::parse_max_pages(response)
    }
}
