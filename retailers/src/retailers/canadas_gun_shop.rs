use std::{pin::Pin, time::Duration};

use async_trait::async_trait;
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
    results::{
        constants::{ActionType, FirearmType, RetailerName},
        firearm::{FirearmPrice, FirearmResult},
    },
    traits::{Retailer, SearchParams},
    utils::{
        conversions::{price_to_cents, string_to_u64},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
        json::{json_get_array, json_get_object},
    },
};

const PAGE_COOLDOWN: u64 = 10;
const PAGE_LIMIT: u64 = 100;
const MAIN_URL: &str = "https://store.theshootingcentre.com/firearms/{firearm_type}/?limit={page_limit}&mode=6&Action+Type={action}&page={page}";
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

    fn get_price(product_element: ElementRef) -> Result<FirearmPrice, RetailerError> {
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

        let mut price = FirearmPrice {
            regular_price: 0,
            sale_price: None,
        };

        if price_non_sale_str == "" {
            price.regular_price = price_to_cents(price_str)?;
        } else {
            price.regular_price = price_to_cents(price_non_sale_str)?;
            price.sale_price = Some(price_to_cents(price_str)?);
        }

        Ok(price)
    }

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

    async fn parse_nested_firearm(
        &self,
        url: String,
        name: String,
        image: String,
        search_param: &SearchParams<'_>,
    ) -> Result<Vec<FirearmResult>, RetailerError> {
        let mut nested_results: Vec<FirearmResult> = Vec::new();

        let request = RequestBuilder::new().set_url(&url).build();
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

            let mut price_obj = json_get_object(&data, "price".into())?;
            price_obj = json_get_object(&price_obj, "without_tax".into())?;
            price_obj = json_get_object(&price_obj, "formatted".into())?;

            let Some(price_str) = price_obj.as_str() else {
                let message = format!("Failed to convert {} into a string", price_obj);
                error!(message);
                return Err(RetailerError::ApiResponseInvalidShape(message));
            };

            let price = FirearmPrice {
                regular_price: price_to_cents(price_str.into())?,
                sale_price: None,
            };

            let formatted_name = format!("{} - {}", name, model_name);

            let mut nested_firearm =
                FirearmResult::new(formatted_name, &url, price, self.get_retailer_name());
            nested_firearm.thumbnail_link = Some(image.to_string());
            nested_firearm.action_type = search_param.action_type;
            nested_firearm.ammo_type = search_param.ammo_type;
            nested_firearm.firearm_class = search_param.firearm_class;
            nested_firearm.firearm_type = search_param.firearm_type;

            nested_results.push(nested_firearm);

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
        search_param: &SearchParams,
    ) -> Result<Request, RetailerError> {
        let firearm_type = match search_param.firearm_type.unwrap() {
            FirearmType::Rifle => "rifles",
            FirearmType::Shotgun => "shotguns",
        };

        let action_type = match search_param.action_type.unwrap() {
            ActionType::SemiAuto => "Semi-Auto",
            ActionType::LeverAction => "Lever",
            ActionType::BreakAction => "Break",
            ActionType::BoltAction => "Bolt",
            ActionType::OverUnder => "",
            ActionType::PumpAction => "Pump",
            ActionType::SideBySide => "",
            ActionType::SingleShot => "",
            ActionType::Revolver => "Revolver",
            ActionType::StraightPull => "Straight-Pull",
        };

        let request: Request = RequestBuilder::new()
            .set_url(
                MAIN_URL
                    .replace("{firearm_type}", firearm_type)
                    .replace("{page_limit}", PAGE_LIMIT.to_string().as_str())
                    .replace("{page}", (page_num + 1).to_string().as_str())
                    .replace("{action}", action_type),
            )
            .build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_param: &SearchParams,
    ) -> Result<Vec<FirearmResult>, RetailerError> {
        let mut firearms: Vec<FirearmResult> = Vec::new();

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

        let mut nested_firearm_handles: Vec<
            Pin<Box<dyn Future<Output = Result<Vec<FirearmResult>, RetailerError>> + Send>>,
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
                nested_firearm_handles.push(Box::pin(
                    self.parse_nested_firearm(url, name, image, search_param)
                        .into_future(),
                ));

                continue;
            }

            let price = Self::get_price(product)?;

            let mut new_firearm =
                FirearmResult::new(name, url, price, RetailerName::CanadasGunShop);
            new_firearm.thumbnail_link = Some(image.to_string());
            new_firearm.action_type = search_param.action_type;
            new_firearm.ammo_type = search_param.ammo_type;
            new_firearm.firearm_class = search_param.firearm_class;
            new_firearm.firearm_type = search_param.firearm_type;

            firearms.push(new_firearm);
        }

        for handler in nested_firearm_handles {
            firearms.append(&mut handler.await?);
        }

        Ok(firearms)
    }

    fn get_search_parameters(&self) -> Result<Vec<SearchParams>, RetailerError> {
        let params = Vec::from_iter([
            // rifles
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::BoltAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::BreakAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::LeverAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::PumpAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::Revolver),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::SemiAuto),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::StraightPull),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            // shotguns
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::BoltAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::BreakAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::LeverAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::PumpAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::Revolver),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::SemiAuto),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::StraightPull),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
        ]);

        Ok(params)
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
