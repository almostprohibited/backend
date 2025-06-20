use std::{pin::Pin, time::Duration};

use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::{
    request::{Request, RequestBuilder},
    traits::{Crawler, HttpMethod},
    unprotected::UnprotectedCrawler,
};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use serde_json::Value;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use urlencoding::encode;

use crate::{
    errors::RetailerError,
    traits::{Retailer, SearchTerm},
    utils::{
        conversions::{price_to_cents, string_to_u64},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
        json::{json_get_array, json_get_object},
    },
};

const PAGE_COOLDOWN: u64 = 10;
const PAGE_LIMIT: u64 = 100;
const MAIN_URL: &str =
    "https://store.theshootingcentre.com/{category}/?limit={page_limit}&mode=6&page={page}";
const API_URL: &str =
    "https://store.theshootingcentre.com/remote/v1/product-attributes/{product_id}";

pub struct CanadasGunShop {
    crawler: UnprotectedCrawler,
    retailer: RetailerName,
}

impl CanadasGunShop {
    pub fn new() -> Self {
        Self {
            crawler: UnprotectedCrawler::new(),
            retailer: RetailerName::CanadasGunShop,
        }
    }

    /// For regular parcing using HTML elements
    fn get_price_from_element(product_element: ElementRef) -> Result<Price, RetailerError> {
        /*
        <span data-product-price-without-tax="" class="price price--withoutTax price--main">$2,160.00</span>
        <span data-product-non-sale-price-without-tax="" class="price price--non-sale">$2,400.00</span>

        <span data-product-non-sale-price-without-tax="" class="price price--non-sale"></span>
        </span> */

        let price_main = extract_element_from_element(product_element, "span.price--main".into())?;
        let price_non_sale =
            extract_element_from_element(product_element, "span.price--non-sale".into())?;

        let price_str = element_to_text(price_main);
        let price_non_sale_str = element_to_text(price_non_sale);

        let mut price = Price {
            regular_price: price_to_cents(price_str)?,
            sale_price: None,
        };

        if !price_non_sale_str.is_empty() {
            price.sale_price = Some(price.regular_price);
            price.regular_price = price_to_cents(price_non_sale_str)?;
        }

        Ok(price)
    }

    /// For nested pricing using the JSON API response
    fn get_price_from_object(obj: &Value) -> Result<Price, RetailerError> {
        // "price": {
        //     "without_tax": {
        //         "formatted": "$3,476.00",
        //         "value": 3476,
        //         "currency": "CAD"
        //     },
        //     "tax_label": "Tax",
        //     "sale_price_without_tax": { <-- not included in non sales
        //         "formatted": "$3,476.00",
        //         "value": 3476,
        //         "currency": "CAD"
        //     },
        //     "non_sale_price_without_tax": { <-- not included in non sales
        //         "formatted": "$3,950.00",
        //         "value": 3950,
        //         "currency": "CAD"
        //     }
        // },

        let mut main_price = json_get_object(obj, "without_tax".into())?;
        main_price = json_get_object(main_price, "formatted".into())?;

        let Some(price_str) = main_price.as_str() else {
            let message = format!("Failed to convert {} into a string", main_price);
            error!(message);
            return Err(RetailerError::ApiResponseInvalidShape(message));
        };

        let mut price = Price {
            regular_price: price_to_cents(price_str.into())?,
            sale_price: None,
        };

        if let Ok(non_sale_price) = json_get_object(obj, "non_sale_price_without_tax".into()) {
            price.sale_price = Some(price.regular_price);

            let regular_price = json_get_object(non_sale_price, "formatted".into())?;
            let Some(regular_price_str) = regular_price.as_str() else {
                let message = format!("Failed to convert {} into a string", regular_price);
                error!(message);
                return Err(RetailerError::ApiResponseInvalidShape(message));
            };

            price.regular_price = price_to_cents(regular_price_str.into())?;
        };

        Ok(price)
    }

    /// Returns Vec<(model_id, model_sub_name)>
    fn get_in_stock_models(&self, result: &String) -> Result<Vec<(String, String)>, RetailerError> {
        let html = Html::parse_document(result);
        let script_selector = Selector::parse("script[type='text/javascript']").unwrap();
        let option_selector =
            Selector::parse("select.form-select--small > option[data-product-attribute-value]")
                .unwrap();

        let mut model_ids_in_stock: Vec<String> = Vec::new();
        let mut models_in_stock: Vec<(String, String)> = Vec::new();

        // I could get the model IDs in a single go, but more reliable
        // to parse the JSON instead of checking if the option says
        // "out of stock"
        for script in html.select(&script_selector) {
            let javascript = element_to_text(script);

            if !javascript.contains("in_stock_attributes") {
                debug!("JSON missing 'in_stock_attributes', skipping");
                continue;
            }

            let json_result = match javascript.split_once(" = ") {
                Some((_, json)) => serde_json::from_str::<Value>(json.trim_end_matches(";"))?,
                None => {
                    let message = format!("Unexpected JS, failed to split variable and value");

                    error!(message);

                    return Err(RetailerError::GeneralError(message));
                }
            };
            let attributes = json_get_object(&json_result, "product_attributes".into())?;
            let in_stock_prop = json_get_object(&attributes, "in_stock_attributes".into())?;
            let in_stock_array = json_get_array(&in_stock_prop)?;

            for in_stock_id in in_stock_array {
                let Some(id) = in_stock_id.as_u64() else {
                    let message = format!("In stock attribute is not a number");

                    error!(message);

                    return Err(RetailerError::GeneralError(message));
                };

                model_ids_in_stock.push(id.to_string());
            }

            break;
        }

        // find the model name
        for option in html.select(&option_selector) {
            let model_id = element_extract_attr(option, "data-product-attribute-value".into())?;
            if model_ids_in_stock.contains(&model_id) {
                models_in_stock.push((model_id, element_to_text(option)));
            }
        }

        info!("Found extra model IDs: {:?}", models_in_stock);

        Ok(models_in_stock)
    }

    async fn parse_nested(
        &self,
        url: String,
        name: String,
        image: String,
        search_term: &SearchTerm,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut nested_results: Vec<CrawlResult> = Vec::new();

        let request = RequestBuilder::new().set_url(url.clone()).build();
        let result = self.crawler.make_web_request(request).await?;

        let models = self.get_in_stock_models(&result)?;

        let (product_id, model_key_name) = {
            let html = Html::parse_document(&result);

            let input_element =
                extract_element_from_element(html.root_element(), "input[name=product_id]".into())?;
            let product_id = element_extract_attr(input_element, "value".into())?;

            let select_element = extract_element_from_element(
                html.root_element(),
                "select.form-select--small".into(),
            )?;
            let mut model_key_name = element_extract_attr(select_element, "name".into())?;
            model_key_name = encode(&model_key_name).into_owned();

            (product_id, model_key_name)
        };

        for (model_id, model_name) in models {
            let body = format!(
                "action=add&{}={}&product_id={}&user=",
                model_key_name, model_id, product_id
            );

            let request = RequestBuilder::new()
                .set_url(API_URL.replace("{product_id}", &product_id))
                .set_method(HttpMethod::POST)
                .set_body(body)
                .build();

            let result = self.crawler.make_web_request(request).await?;

            let json = serde_json::from_str::<Value>(result.as_str())?;
            let data = json_get_object(&json, "data".into())?;

            let price_obj = json_get_object(&data, "price".into())?;
            let price = Self::get_price_from_object(price_obj)?;

            let formatted_name = format!("{} - {}", name, model_name);

            let new_result = CrawlResult::new(
                formatted_name,
                url.clone(),
                price,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(image.to_string());

            nested_results.push(new_result);

            sleep(Duration::from_secs(self.get_page_cooldown())).await;
        }

        Ok(nested_results)
    }
}

#[async_trait]
impl Retailer for CanadasGunShop {
    fn get_retailer_name(&self) -> RetailerName {
        self.retailer
    }

    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &SearchTerm,
    ) -> Result<Request, RetailerError> {
        let request: Request = RequestBuilder::new()
            .set_url(
                MAIN_URL
                    .replace("{category}", &search_term.term)
                    .replace("{page_limit}", PAGE_LIMIT.to_string().as_str())
                    .replace("{page}", (page_num + 1).to_string().as_str()),
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

        // commit another Rust sin, and clone the entire HTML
        // as a string since scraper::ElementRef is not thread safe
        // we'll recreate the Node later
        let products = {
            let html = Html::parse_document(response);
            let product_selector = Selector::parse("li.product > article.card").unwrap();
            html.select(&product_selector)
                .map(|element| element.html().clone())
                .collect::<Vec<_>>()
        };

        let mut nested_handlers: Vec<
            Pin<Box<dyn Future<Output = Result<Vec<CrawlResult>, RetailerError>> + Send>>,
        > = Vec::new();

        for doc in products {
            let product_inner = Html::parse_document(&doc);
            let product = product_inner.root_element();

            let name_link_element =
                extract_element_from_element(product, "h4.card-title > a".into())?;

            let image_element = extract_element_from_element(
                product,
                "div.card-img-container > img.card-image".into(),
            )?;

            let url = element_extract_attr(name_link_element, "href".into())?;
            let name = element_to_text(name_link_element);
            let image = element_extract_attr(image_element, "src".into())?;

            let price_element = extract_element_from_element(product, "span.price--main".into())?;

            if price_element
                .text()
                .collect::<String>()
                .trim()
                .contains("-")
            {
                // this is a nested firearm, there are models inside
                // the URL that have different prices
                nested_handlers.push(Box::pin(
                    self.parse_nested(url, name, image, search_term)
                        .into_future(),
                ));

                continue;
            }

            let price = Self::get_price_from_element(product)?;

            let new_result = CrawlResult::new(
                name,
                url,
                price,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(image.to_string());

            results.push(new_result);
        }

        for handler in nested_handlers {
            results.append(&mut handler.await?);
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
                term: "optics".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "reloading".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "gun-parts-accessories".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "optics-accessories".into(),
                category: Category::Other,
            },
        ])
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(response);

        let Ok(count_element) =
            extract_element_from_element(html.root_element(), "div.pagination-info".into())
        else {
            warn!("Page does not contain extra pages, returning 0 as max page");
            return Ok(0);
        };

        let count_text = element_to_text(count_element);
        let regex = Regex::new(r"(\d+)\s+total$").unwrap();

        // Regex::new(r"(?<=of )\d+(?= total)")
        // look around operations are not supported by the regex crate
        // https://crates.io/crates/fancy-regex might work, but provides
        // no guarantees on safety
        // oh well

        let Some(item_counts) = regex.captures(&count_text) else {
            let message = format!("Failed to extract total item count from: {}", count_text);

            error!(message);

            return Err(RetailerError::GeneralError(message));
        };

        let Some(item_count) = item_counts.get(1) else {
            // this should never fail as the regex always has a single match
            // but check anyways in case I change it and forget

            let message = format!(
                "Capture group does not contain expected match: {:?}",
                item_counts
            );

            error!(message);

            return Err(RetailerError::GeneralError(message));
        };

        debug!("{:?}", item_count);

        let item_as_int = string_to_u64(item_count.as_str().into())?;

        Ok((item_as_int / PAGE_LIMIT).into())
    }

    fn get_crawler(&self) -> UnprotectedCrawler {
        self.crawler
    }

    fn get_page_cooldown(&self) -> u64 {
        PAGE_COOLDOWN
    }
}
