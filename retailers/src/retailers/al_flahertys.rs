use std::{pin::Pin, time::Duration};

use async_trait::async_trait;
use crawler::{
    request::{Request, RequestBuilder},
    traits::{Crawler, HttpMethod},
    unprotected::UnprotectedCrawler,
};
use scraper::{Html, Selector};
use serde_json::Value;
use tokio::time::sleep;
use tracing::{error, info};
use urlencoding::encode;

use crate::{
    errors::RetailerError,
    results::{
        constants::{ActionType, AmmunitionType, FirearmType, RetailerName},
        firearm::{FirearmPrice, FirearmResult},
    },
    traits::{Retailer, SearchParams},
    utils::{
        conversions::price_to_cents,
        html::{element_extract_attr, element_to_text, extract_element_from_element},
        json::{json_get_array, json_get_object},
    },
};

const PAGE_COOLDOWN: u64 = 10;
const PAGE_LIMIT: u64 = 36;
const AL_FLAHERTYS_KLEVU_API_KEY: &str = "klevu-170966446878517137";
const MAIN_URL: &str = "https://uscs33v2.ksearchnet.com/cs/v2/search";
const MAIN_PAYLOAD: &str = "{\"context\":{\"apiKeys\":[\"{api_key}\"]},\"recordQueries\":[{\"id\":\"productList\",\"typeOfRequest\":\"CATNAV\",\"settings\":{\"query\":{\"term\":\"*\",\"categoryPath\":\"Shooting Supplies, Firearms & Ammunition;Firearms\"},\"typeOfRecords\":[\"KLEVU_PRODUCT\"],\"offset\":{offset},\"limit\":\"{page_limit}\",\"typeOfSearch\":\"AND\",\"priceFieldSuffix\":\"CAD\"},\"filters\":{\"filtersToReturn\":{\"enabled\":true,\"options\":{\"limit\":50},\"rangeFilterSettings\":[{\"key\":\"klevu_price\",\"minMax\":\"true\"}]},\"applyFilters\":{\"filters\":[{\"key\":\"type\",\"values\":[\"{category}\"]}]}}}]}";
const API_URL: &str = "https://alflahertys.com/remote/v1/product-attributes/{product_id}";

pub struct AlFlahertys {
    crawler: UnprotectedCrawler,
}

impl AlFlahertys {
    pub fn new() -> Self {
        Self {
            crawler: UnprotectedCrawler::new(),
        }
    }

    fn get_result(api_response: &String) -> Result<Value, RetailerError> {
        let json = serde_json::from_str::<Value>(api_response.as_str())?;

        // we can't deserialize this properly in Rust since
        // either Al Flahertys, or Klevu, has a mix of different cases and formats for their keys
        // and they also just exclude keys if they are optional in the response
        let result_array_obj = json_get_object(&json, "queryResults".into())?;
        let result_array = json_get_array(result_array_obj)?;

        let Some(result) = result_array.first() else {
            error!("Empty records\n{:?}", result_array);
            return Err(RetailerError::ApiResponseInvalidShape(
                "Empty records array".into(),
            ));
        };

        Ok(result.clone())
    }

    fn value_to_string(json: &Value, key: &str) -> Result<String, RetailerError> {
        let prop = json_get_object(json, key.into())?;

        let Some(string_repr) = prop.as_str() else {
            let message = format!("{} is not a string: {}", key, prop);

            error!(message);

            return Err(RetailerError::ApiResponseInvalidShape(message));
        };

        Ok(string_repr.to_string())
    }

    fn get_in_stock_models(&self, result: &String) -> Result<Vec<(String, String)>, RetailerError> {
        let html = Html::parse_document(result);
        let option_selector =
            Selector::parse("select.form-select--small > option[data-product-attribute-value]")
                .unwrap();

        let mut models_in_stock: Vec<(String, String)> = Vec::new();

        for option in html.select(&option_selector) {
            let model_id = element_extract_attr(option, "data-product-attribute-value".into())?;

            models_in_stock.push((model_id, element_to_text(option)));
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
                "action=add&product_id={}&{}={}&qty%5B%5D=1",
                product_id, model_key_name, model_id
            );

            sleep(Duration::from_secs(self.get_page_cooldown())).await;
            let request = RequestBuilder::new()
                .set_url(API_URL.replace("{product_id}", &product_id))
                .set_method(HttpMethod::POST)
                .set_body(body)
                .build();

            let result = self.crawler.make_web_request(request).await?;

            let json = serde_json::from_str::<Value>(result.as_str())?;
            let data = json_get_object(&json, "data".into())?;

            if json_get_object(&data, "stock".into())? == "0" {
                continue;
            }

            let mut price_obj = json_get_object(&data, "price".into())?;
            price_obj = json_get_object(&price_obj, "without_tax".into())?;
            price_obj = json_get_object(&price_obj, "formatted".into())?;

            let Some(price_str) = price_obj.as_str() else {
                let message = format!("Failed to convert {} into a string", price_obj);
                error!(message);
                return Err(RetailerError::ApiResponseInvalidShape(message));
            };

            let Some((_, price_without_currency)) = price_str.split_once(" ") else {
                let message = format!(
                    "Expected price in format: 'CAD $1.23', got {} instead",
                    price_obj
                );
                error!(message);
                return Err(RetailerError::ApiResponseInvalidShape(message));
            };

            let price = FirearmPrice {
                regular_price: price_to_cents(price_without_currency.into())?,
                sale_price: None,
            };

            let mut formatted_name = name.replace("Various Calibers", &model_name);

            if formatted_name == name {
                // this indicates that original text did not have "Various Calibers"
                formatted_name = format!("{} - {}", name, model_name);
            }

            let mut nested_firearm =
                FirearmResult::new(formatted_name, &url, price, RetailerName::CanadasGunShop);
            nested_firearm.thumbnail_link = Some(image.to_string());
            nested_firearm.action_type = search_param.action_type;
            nested_firearm.ammo_type = search_param.ammo_type;
            nested_firearm.firearm_class = search_param.firearm_class;
            nested_firearm.firearm_type = search_param.firearm_type;

            nested_results.push(nested_firearm);
        }

        Ok([].into())
    }
}

#[async_trait]
impl Retailer for AlFlahertys {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_param: &SearchParams,
    ) -> Result<Request, RetailerError> {
        let offset = PAGE_LIMIT * page_num;

        let body = MAIN_PAYLOAD
            .replace("{api_key}", AL_FLAHERTYS_KLEVU_API_KEY)
            .replace("{category}", search_param.lookup)
            .replace("{offset}", offset.to_string().as_str())
            .replace("{page_limit}", PAGE_LIMIT.to_string().as_str());

        let Ok(json) = serde_json::from_str::<Value>(body.as_str()) else {
            error!("Failed to serialize payload");
            return Err(RetailerError::InvalidRequestBody(body));
        };

        let request = RequestBuilder::new()
            .set_url(MAIN_URL)
            .set_json_body(json)
            .set_method(HttpMethod::POST)
            .build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_param: &SearchParams,
    ) -> Result<Vec<FirearmResult>, RetailerError> {
        let result = Self::get_result(response)?;

        let records = json_get_object(&result, "records".into())?;
        let firearms_json = json_get_array(records)?;

        let mut nested_handlers: Vec<
            Pin<Box<dyn Future<Output = Result<Vec<FirearmResult>, RetailerError>> + Send>>,
        > = Vec::new();
        let mut firearms: Vec<FirearmResult> = Vec::new();

        for firearm in firearms_json {
            if Self::value_to_string(firearm, "inStock")? != "yes" {
                continue;
            }

            let url = Self::value_to_string(firearm, "url")?;
            let name = Self::value_to_string(firearm, "name")?;
            let image = Self::value_to_string(firearm, "imageUrl")?;

            let base_price_string = Self::value_to_string(firearm, "basePrice")?;
            let sale_price_string = Self::value_to_string(firearm, "salePrice")?;

            let variant_value = json_get_object(firearm, "totalVariants".into())?;
            let Some(variants) = variant_value.as_u64() else {
                let message = format!("Failed to convert {} into an u64", variant_value);
                error!(message);
                return Err(RetailerError::ApiResponseInvalidShape(message));
            };

            if variants != 0 {
                nested_handlers.push(Box::pin(
                    self.parse_nested_firearm(url, name, image, search_param)
                        .into_future(),
                ));

                continue;
            }

            let mut price = FirearmPrice {
                regular_price: price_to_cents(base_price_string.clone())?,
                sale_price: None,
            };

            if base_price_string != sale_price_string {
                price.sale_price = Some(price_to_cents(sale_price_string)?);
            }

            let mut new_firearm = FirearmResult::new(name, url, price, RetailerName::AlFlahertys);
            new_firearm.thumbnail_link = Some(image);
            new_firearm.action_type = search_param.action_type;
            new_firearm.ammo_type = search_param.ammo_type;
            new_firearm.firearm_class = search_param.firearm_class;
            new_firearm.firearm_type = search_param.firearm_type;

            firearms.push(new_firearm);
        }

        for handler in nested_handlers {
            firearms.append(&mut handler.await?);
        }

        Ok(firearms)
    }

    fn get_search_parameters(&self) -> Result<Vec<SearchParams>, RetailerError> {
        /*
        Muzzle Loader
        Firearm - Combination Gun
        */

        let search_params = Vec::from_iter([
            SearchParams {
                lookup: "Rifle - Bolt Action",
                action_type: Some(ActionType::BoltAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "Rifle - Lever Action",
                action_type: Some(ActionType::LeverAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "Rifle - Rimfire",
                action_type: None,
                ammo_type: Some(AmmunitionType::Rimfire),
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "Rifle - Semi",
                action_type: Some(ActionType::SemiAuto),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "Rifle Single Shot",
                action_type: Some(ActionType::SingleShot),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "Shotgun - Break Action",
                action_type: Some(ActionType::BoltAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "Shotgun - Semi",
                action_type: Some(ActionType::SemiAuto),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "Shotgun - Pump Action",
                action_type: Some(ActionType::PumpAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "Shotgun - Lever Action",
                action_type: Some(ActionType::LeverAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "Shotgun - Bolt Action",
                action_type: Some(ActionType::BoltAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
        ]);

        Ok(search_params)
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let result = Self::get_result(response)?;

        let meta = json_get_object(&result, "meta".into())?;
        let total_results = json_get_object(meta, "totalResultsFound".into())?;

        let Some(count) = total_results.as_u64() else {
            let message = format!("Failed to parse count as number: {}", total_results);

            error!(message);

            return Err(RetailerError::ApiResponseInvalidShape(message.into()));
        };

        Ok(count / 36 + 1)
    }

    fn get_crawler(&self) -> UnprotectedCrawler {
        self.crawler
    }
    fn get_page_cooldown(&self) -> u64 {
        PAGE_COOLDOWN
    }
}
