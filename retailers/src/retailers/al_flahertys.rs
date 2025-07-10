use std::{collections::HashMap, pin::Pin, time::Duration};

use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::{
    request::{Request, RequestBuilder},
    traits::HttpMethod,
    unprotected::UnprotectedCrawler,
};
use scraper::{Html, Selector};
use serde_json::Value;
use tokio::time::sleep;
use tracing::{debug, error, info};
use urlencoding::encode;

use crate::{
    errors::RetailerError,
    pagination_client::CRAWL_COOLDOWN_SECS,
    traits::{Retailer, SearchTerm},
    utils::{
        conversions::price_to_cents,
        ecommerce::bigcommerce_nested::BigCommerceNested,
        html::{element_extract_attr, element_to_text, extract_element_from_element},
        json::{json_get_array, json_get_object},
    },
};

const COOKIE_NAME: &str = "Shopper-Pref";
const PAGE_LIMIT: u64 = 36;
const AL_FLAHERTYS_KLEVU_API_KEY: &str = "klevu-170966446878517137";
const MAIN_URL: &str = "https://uscs33v2.ksearchnet.com/cs/v2/search";
const MAIN_PAYLOAD: &str = "{\"context\":{\"apiKeys\":[\"{api_key}\"]},\"recordQueries\":[{\"id\":\"productList\",\"typeOfRequest\":\"CATNAV\",\"settings\":{\"query\":{\"term\":\"*\",\"categoryPath\":\"{category}\"},\"typeOfRecords\":[\"KLEVU_PRODUCT\"],\"offset\":{offset},\"limit\":\"{page_limit}\",\"priceFieldSuffix\":\"CAD\"},\"filters\":{\"filtersToReturn\":{\"enabled\":true,\"options\":{\"limit\":50},\"rangeFilterSettings\":[{\"key\":\"klevu_price\",\"minMax\":\"true\"}]}}}]}";
const API_URL: &str = "https://alflahertys.com/remote/v1/product-attributes/{product_id}";

pub struct AlFlahertys {
    crawler: UnprotectedCrawler,
    retailer: RetailerName,
}

impl AlFlahertys {
    pub fn new() -> Self {
        Self {
            crawler: UnprotectedCrawler::new(),
            retailer: RetailerName::AlFlahertys,
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

    /// Theres no way to filter by in-stock models, we have to fetch them all
    fn get_in_stock_models(&self, result: &String) -> Result<Vec<(String, String)>, RetailerError> {
        let html = Html::parse_document(result);
        let option_selector =
            Selector::parse("section.productView-details select.form-select--small > option[data-product-attribute-value]")
                .unwrap();

        let mut models: Vec<(String, String)> = Vec::new();

        for option in html.select(&option_selector) {
            let model_id = element_extract_attr(option, "data-product-attribute-value")?;

            models.push((model_id, element_to_text(option)));
        }

        info!("Found extra model IDs: {:?}", models);

        Ok(models)
    }

    // the cookie rotates on each request, need to follow cookie crumbs
    fn get_shopper_pref_cookie(cookies: &HashMap<String, String>) -> Result<String, RetailerError> {
        let Some(pref_value) = cookies.get(COOKIE_NAME) else {
            let message =
                "Failed to fetch main product correctly, response is missing Shopper-Pref cookie";
            error!(message);
            return Err(RetailerError::ApiResponseInvalidShape(message.into()));
        };

        Ok(pref_value.clone())
    }

    // alflahertys works weirdly
    // you need to set ?setCurrencyId=1 on the main product page
    // then you need to copy the Shopper-Pref cookie from the
    // main page response and add it to the API request to get CAD pricing
    async fn parse_nested_firearm(
        &self,
        url: String,
        name: String,
        image: String,
        search_param: &SearchTerm,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let cad_formatted_pricing = format!("{}?setCurrencyId=1", url);

        let request = RequestBuilder::new()
            .set_url(&cad_formatted_pricing)
            .build();
        let result = self.crawler.make_web_request(request).await?;

        let models = self.get_in_stock_models(&result.body)?;

        let (product_id, model_key_name) = {
            let html = Html::parse_document(&result.body);

            let input_element =
                extract_element_from_element(html.root_element(), "input[name=product_id]")?;
            let product_id = element_extract_attr(input_element, "value")?;

            let select_element =
                extract_element_from_element(html.root_element(), "select.form-select--small")?;
            let mut model_key_name = element_extract_attr(select_element, "name")?;
            model_key_name = encode(&model_key_name).into_owned();

            (product_id, model_key_name)
        };

        let mut shopper_cookie_value = Self::get_shopper_pref_cookie(&result.cookies)?;

        for (model_id, model_name) in models {
            let body = format!(
                "action=add&product_id={}&{}={}&qty%5B%5D=1",
                product_id, model_key_name, model_id
            );

            sleep(Duration::from_secs(CRAWL_COOLDOWN_SECS)).await;

            let cookie = format!("{COOKIE_NAME}={shopper_cookie_value};");
            debug!("Using cookie {cookie}");

            let request = RequestBuilder::new()
                .set_url(API_URL.replace("{product_id}", &product_id))
                .set_method(HttpMethod::POST)
                .set_headers(
                    &[
                        ("Cookie".to_string(), cookie),
                        (
                            "Content-Type".to_string(),
                            "application/x-www-form-urlencoded".to_string(),
                        ),
                    ]
                    .to_vec(),
                )
                .set_body(body)
                .build();

            let nested_web_result = self.crawler.make_web_request(request).await?;
            shopper_cookie_value = Self::get_shopper_pref_cookie(&nested_web_result.cookies)?;

            let json = serde_json::from_str::<Value>(&nested_web_result.body)?;
            let data = json_get_object(&json, "data".into())?;

            // boolean check since I am comparing against `Value`
            if json_get_object(&data, "instock".into())? == false {
                continue;
            }

            let price = BigCommerceNested::get_price_from_object(data)?;
            let formatted_name = format!("({}) - {}", model_name, name);

            let new_result = CrawlResult::new(
                formatted_name,
                url.clone(),
                price,
                self.get_retailer_name(),
                search_param.category,
            )
            .with_image_url(image.to_string());

            results.push(new_result);
        }

        Ok(results)
    }
}

#[async_trait]
impl Retailer for AlFlahertys {
    fn get_retailer_name(&self) -> RetailerName {
        self.retailer
    }

    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &SearchTerm,
    ) -> Result<Request, RetailerError> {
        let offset = PAGE_LIMIT * page_num;

        let body = MAIN_PAYLOAD
            .replace("{api_key}", AL_FLAHERTYS_KLEVU_API_KEY)
            .replace("{category}", &search_term.term)
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
        search_term: &SearchTerm,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let result = Self::get_result(response)?;

        let records = json_get_object(&result, "records".into())?;
        let item_objects = json_get_array(records)?;

        let mut nested_handlers: Vec<
            Pin<Box<dyn Future<Output = Result<Vec<CrawlResult>, RetailerError>> + Send>>,
        > = Vec::new();

        let mut results: Vec<CrawlResult> = Vec::new();

        for item in item_objects {
            if Self::value_to_string(item, "inStock")? != "yes" {
                continue;
            }

            let url = Self::value_to_string(item, "url")?;
            let name = Self::value_to_string(item, "name")?;
            let image = Self::value_to_string(item, "imageUrl")?;

            let base_price_string = Self::value_to_string(item, "basePrice")?;
            let sale_price_string = Self::value_to_string(item, "salePrice")?;

            let variant_value = json_get_object(item, "totalVariants".into())?;
            let Some(variants) = variant_value.as_u64() else {
                let message = format!("Failed to convert {} into an u64", variant_value);
                error!(message);
                return Err(RetailerError::ApiResponseInvalidShape(message));
            };

            if variants != 0 {
                nested_handlers.push(Box::pin(
                    self.parse_nested_firearm(url, name, image, search_term)
                        .into_future(),
                ));

                continue;
            }

            let mut price = Price {
                regular_price: price_to_cents(base_price_string.clone())?,
                sale_price: None,
            };

            if base_price_string != sale_price_string {
                price.sale_price = Some(price_to_cents(sale_price_string)?);
            }

            let new_result = CrawlResult::new(
                name,
                url,
                price,
                RetailerName::AlFlahertys,
                search_term.category,
            )
            .with_image_url(image);

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
                term: "Shooting Supplies, Firearms & Ammunition;Firearms".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "Shooting Supplies, Firearms & Ammunition;Stocks, Parts, Barrels & Kits"
                    .into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "Shooting Supplies, Firearms & Ammunition;Shooting Accessories".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "Shooting Supplies, Firearms & Ammunition;Storage & Transportation".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "Optics".into(),
                category: Category::Other,
            },
        ])
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

        Ok(count / PAGE_LIMIT + 1)
    }
}
