use std::collections::HashMap;

use async_trait::async_trait;
use base64::{Engine, prelude::BASE64_STANDARD};
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::{
    request::{Request, RequestBuilder},
    unprotected::UnprotectedCrawler,
};
use futures::executor;
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use tracing::{debug, trace};

use crate::{
    errors::RetailerError,
    traits::{Retailer, SearchTerm},
    utils::{
        conversions::{price_to_cents, string_to_u64},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const BASE_URL: &str = "https://www.gotenda.com/";
const URL: &str = "https://www.gotenda.com/product-category/{category}/page/{page}/?stock=instock";

pub struct Tenda {
    retailer: RetailerName,
    securi_cookie: String,
}

impl Tenda {
    pub fn new() -> Result<Self, RetailerError> {
        let cookie = executor::block_on(Self::set_securi_cookie())?;

        debug!("Using cookie: {cookie}");

        Ok(Self {
            retailer: RetailerName::Tenda,
            securi_cookie: cookie,
        })
    }

    fn unwrap_regex_capture(regex: &Regex, haystack: &str) -> Result<String, RetailerError> {
        let Some(captures) = regex.captures(haystack) else {
            return Err(RetailerError::GeneralError(format!(
                "Failed to search for {} inside of {}",
                regex.as_str(),
                haystack
            )));
        };

        let Some(result) = captures.get(1) else {
            return Err(RetailerError::GeneralError(format!(
                "Invalid return capture group (should not be possible) for {}",
                regex.as_str()
            )));
        };

        Ok(result.as_str().to_string())
    }

    fn get_cookie_name(haystack: &str) -> Result<String, RetailerError> {
        let cookie_name_regex = Regex::new(r##";document\.cookie=(.*?)\+\s*\"=\"\s*\+"##)
            .expect("Regex should compile as nothing has changed");

        let cookie_name_obfuscated = Self::unwrap_regex_capture(&cookie_name_regex, &haystack)?;
        let mut cookie_name_parts: Vec<String> = Vec::new();

        for cooke_name_part in cookie_name_obfuscated.split("+") {
            let Some(individual_char) = cooke_name_part.get(1..2) else {
                return Err(RetailerError::GeneralError(format!(
                    "Failed to map value: {}",
                    cooke_name_part
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
        let cookie_value_obfuscated =
            Self::unwrap_regex_capture(&obfuscated_string_regex, haystack)?;

        let mut reconstructed_parts: Vec<String> = Vec::new();

        let char_code_parts: Vec<&str> = cookie_value_obfuscated.split(" + ").collect();

        for part in char_code_parts {
            let Ok(char_code) = Self::unwrap_regex_capture(&char_code_regex, part) else {
                let Some(individual_char) = part.get(1..2) else {
                    return Err(RetailerError::GeneralError(format!(
                        "Captured non String.fromCharCode, but failed to map to char: {}",
                        part
                    )));
                };

                reconstructed_parts.push(individual_char.to_string());
                continue;
            };

            let Ok(char_code) = char_code.parse::<u32>() else {
                return Err(RetailerError::GeneralError(format!(
                    "Char code is not a number: {}",
                    char_code
                )));
            };

            let Some(parsed_char) = char::from_u32(char_code) else {
                return Err(RetailerError::GeneralError(format!(
                    "Failed to convert char into valid UTF-8: {}",
                    char_code
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

        let base64 = Self::unwrap_regex_capture(&base64_regex, &result.body)?;

        trace!("{base64}");

        let Ok(decoded_base64) = BASE64_STANDARD.decode(&base64) else {
            return Err(RetailerError::GeneralError(format!(
                "Failed to decode base64, got this instead: {}",
                base64
            )));
        };

        let Ok(decoded_string) = String::from_utf8(decoded_base64) else {
            return Err(RetailerError::GeneralError(
                "Invalid string, decoded base64 did not convert into a string".to_string(),
            ));
        };

        let cookie_name = Self::get_cookie_name(&decoded_string)?;
        let cookie_value = Self::get_cookie_value(&decoded_string)?;

        Ok(format!("{}={};", cookie_name, cookie_value))
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
impl Retailer for Tenda {
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
        search_term: &SearchTerm,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let fragment = Html::parse_document(&response);

        let product_selector = Selector::parse("ul.products > li.product").unwrap();

        for element in fragment.select(&product_selector) {
            let title_element = extract_element_from_element(element, "h3.products-title > a")?;

            let product_url = element_extract_attr(title_element, "href")?;
            let product_name = element_to_text(title_element);

            let price = Self::get_price(element)?;

            let image_element =
                extract_element_from_element(element, "figure.products-img > a > img")?;
            let image_url = element_extract_attr(image_element, "data-src")?;

            let result = CrawlResult::new(
                product_name,
                product_url,
                price,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(image_url.to_string());

            results.push(result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<SearchTerm> {
        Vec::from_iter([
            SearchTerm {
                term: "firearms/handguns".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "firearms/restricted-rifles".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "firearms/rifles".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "firearms/shotguns".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "firearms/surplus-military".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "firearms/consignment".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "firearms/laser-training".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/magpul-section".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/gun-maintenance-tools".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/gun-maintenance-tools".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/gun-parts/mdt-parts".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/gun-parts/gun-stocks".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/gun-parts/for-shotgun".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/gun-parts/for-revolver".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/gun-parts/parts-for-ruger-1022".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/gun-parts/parts-for-glock".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "product-category/accessories/gun-parts/gun-barrels".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/gun-parts/ar-parts".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/gun-parts/cz-parts".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/gun-parts/upgrade-triggers".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/gun-parts/muzzle-brakes".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/gun-parts/sks-parts".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/bipod-grips-shooting-rest-sling".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/ipsc-3guns/holster".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/ipsc-3guns/trap-skeet".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/ipsc-3guns/pouch".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/ipsc-3guns/belt".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/storage-transport".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/shooting-protection".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/targets".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories/magazines".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "reloading/gun-powders".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "reloading/primers/shotshell-primers".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "reloading/primers/pistol-primers".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "reloading/primers/rifle-primers".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "reloading/tools-accessories".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "reloading/dillon-precision".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "reloading/lee-precision".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "reloading/lyman-mark-7".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "reloading/dies-press".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "reloading/brass-bullet".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "reloading/brass-cleaning".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "optic/binocular-range-finder".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "optic/replacement-sights".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "optic/scope".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "optic/optic-accessories/ringsmount".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "optic/optic-accessories/scope-cover".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "optic/optic-accessories/clean-maintain".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "optic/red-dot".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "optic/nightforce-section/scope-nightforce-section".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "optic/nightforce-section/rings-mounts".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "optic/laser-flashlight".into(),
                category: Category::Other,
            },
        ])
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let fragment = Html::parse_document(&response);
        let page_number_selector =
            Selector::parse("ul.page-numbers > li > a:not(.next):not(.prev).page-numbers").unwrap();

        let page_links = fragment.select(&page_number_selector);

        let Some(last_page_element) = page_links.last() else {
            return Ok(0);
        };

        Ok(string_to_u64(element_to_text(last_page_element))?)
    }
}
