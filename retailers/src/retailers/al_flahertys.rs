use async_trait::async_trait;
use crawler::{
    request::{Request, RequestBuilder},
    traits::HttpMethod,
    unprotected::UnprotectedCrawler,
};
use serde_json::Value;
use tracing::error;

use crate::{
    errors::RetailerError,
    results::{
        constants::{ActionType, AmmunitionType, FirearmType, RetailerName},
        firearm::{FirearmPrice, FirearmResult},
    },
    traits::{Retailer, SearchParams},
    utils::price_to_cents,
};

const PAGE_COOLDOWN: u64 = 10;
const PAGE_LIMIT: u64 = 36;
const AL_FLAHERTYS_KLEVU_API_KEY: &str = "klevu-170966446878517137";
const URL: &str = "https://uscs33v2.ksearchnet.com/cs/v2/search";
const API_PAYLOAD: &str = "{\"context\":{\"apiKeys\":[\"{api_key}\"]},\"recordQueries\":[{\"id\":\"productList\",\"typeOfRequest\":\"CATNAV\",\"settings\":{\"query\":{\"term\":\"*\",\"categoryPath\":\"Shooting Supplies, Firearms & Ammunition;Firearms\"},\"typeOfRecords\":[\"KLEVU_PRODUCT\"],\"offset\":{offset},\"limit\":\"{page_limit}\",\"typeOfSearch\":\"AND\",\"priceFieldSuffix\":\"CAD\"},\"filters\":{\"filtersToReturn\":{\"enabled\":true,\"options\":{\"limit\":50},\"rangeFilterSettings\":[{\"key\":\"klevu_price\",\"minMax\":\"true\"}]},\"applyFilters\":{\"filters\":[{\"key\":\"type\",\"values\":[\"{category}\"]}]}}}]}";

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
        let Ok(json) = serde_json::from_str::<Value>(api_response.as_str()) else {
            error!("Failed to serialize API response");
            return Err(RetailerError::InvalidRequestBody(api_response.to_string()));
        };

        // we can't deserialize this properly in Rust since
        // either Al Flahertys, or Klevu, has a mix of different cases and formats for their keys
        // and they also just exclude keys if they are optional in the response
        let Some(result_array_obj) = json.get("queryResults") else {
            error!("Failed to extract queryResults from response\n{}", json);
            return Err(RetailerError::ApiResponseMissingKey("queryResults".into()));
        };

        let Some(result_array) = result_array_obj.as_array() else {
            error!("queryResults is not an array\n{}", json);
            return Err(RetailerError::ApiResponseInvalidShape(
                "queryResults is not an array".into(),
            ));
        };

        let Some(result) = result_array.first() else {
            error!("Empty records\n{:?}", result_array);
            return Err(RetailerError::ApiResponseInvalidShape(
                "Empty records array".into(),
            ));
        };

        Ok(result.clone())
    }

    fn value_to_string(json: &Value, key: &str) -> Result<String, RetailerError> {
        let Some(prop) = json.get(key) else {
            error!("Failed to find key {}\n{}", key, json);
            return Err(RetailerError::ApiResponseMissingKey(key.into()));
        };

        let Some(string_repr) = prop.as_str() else {
            let message = format!("{} is not a string: {}", key, prop);

            error!(message);

            return Err(RetailerError::ApiResponseInvalidShape(message));
        };

        Ok(string_repr.to_string())
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

        let body = API_PAYLOAD
            .replace("{api_key}", AL_FLAHERTYS_KLEVU_API_KEY)
            .replace("{category}", search_param.lookup)
            .replace("{offset}", offset.to_string().as_str())
            .replace("{page_limit}", PAGE_LIMIT.to_string().as_str());

        let Ok(json) = serde_json::from_str::<Value>(body.as_str()) else {
            error!("Failed to serialize payload");
            return Err(RetailerError::InvalidRequestBody(body));
        };

        let request = RequestBuilder::new()
            .set_url(URL)
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

        let Some(records) = result.get("records") else {
            error!("Failed to extract records from result\n{}", response);
            return Err(RetailerError::ApiResponseMissingKey("records".into()));
        };

        let Some(firearms_json) = records.as_array() else {
            error!("record is not an array\n{}", records);
            return Err(RetailerError::ApiResponseInvalidShape(
                "record is not an array".into(),
            ));
        };

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

        let Some(meta) = result.get("meta") else {
            error!("Failed to extract meta from result\n{}", result);
            return Err(RetailerError::ApiResponseMissingKey("meta".into()));
        };

        let Some(total_results) = meta.get("totalResultsFound") else {
            error!("Failed to extract totalResultsFound from meta\n{}", meta);
            return Err(RetailerError::ApiResponseMissingKey(
                "totalResultsFound".into(),
            ));
        };

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
