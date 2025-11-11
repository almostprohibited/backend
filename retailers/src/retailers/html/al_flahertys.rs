use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::{
    request::{Request, RequestBuilder},
    traits::HttpMethod,
};
use serde::Deserialize;
use serde_json::Value;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        conversions::price_to_cents,
        ecommerce::{BigCommerce, BigCommerceNested},
    },
};

const PAGE_LIMIT: u64 = 36;
const AL_FLAHERTYS_KLEVU_API_KEY: &str = "klevu-170966446878517137";
const MAIN_URL: &str = "https://uscs33v2.ksearchnet.com/cs/v2/search";
const MAIN_PAYLOAD: &str = "{\"context\":{\"apiKeys\":[\"{api_key}\"]},\"recordQueries\":[{\"id\":\"productList\",\"typeOfRequest\":\"CATNAV\",\"settings\":{\"query\":{\"term\":\"*\",\"categoryPath\":\"{category}\"},\"typeOfRecords\":[\"KLEVU_PRODUCT\"],\"offset\":{offset},\"limit\":\"{page_limit}\",\"priceFieldSuffix\":\"CAD\"},\"filters\":{\"filtersToReturn\":{\"enabled\":true,\"options\":{\"limit\":50},\"rangeFilterSettings\":[{\"key\":\"klevu_price\",\"minMax\":\"true\"}]}}}]}";
const SITE_URL: &str = "https://alflahertys.com/";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiResponse {
    // should only be one result
    query_results: Vec<ApiResult>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiResult {
    meta: ApiMeta,
    records: Vec<ApiRecord>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiMeta {
    total_results_found: u64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiRecord {
    image_url: String,
    in_stock: String,
    currency: String,
    base_price: String,
    sale_price: String,
    total_variants: u64,
    url: String,
    name: String,
}

pub struct AlFlahertys {}

impl Default for AlFlahertys {
    fn default() -> Self {
        Self::new()
    }
}

impl AlFlahertys {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for AlFlahertys {}

impl Retailer for AlFlahertys {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::AlFlahertys
    }
}

#[async_trait]
impl HtmlRetailer for AlFlahertys {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let offset = PAGE_LIMIT * page_num;

        let body = MAIN_PAYLOAD
            .replace("{api_key}", AL_FLAHERTYS_KLEVU_API_KEY)
            .replace("{category}", &search_term.term)
            .replace("{offset}", offset.to_string().as_str())
            .replace("{page_limit}", PAGE_LIMIT.to_string().as_str());

        let json = serde_json::from_str::<Value>(body.as_str())?;

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
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut bigcommerce = BigCommerce::new();
        let mut results: Vec<CrawlResult> = Vec::new();

        let response = serde_json::from_str::<ApiResponse>(response)?;

        let Some(query_results) = response.query_results.first() else {
            return Ok(results);
        };

        for product in &query_results.records {
            if product.in_stock.to_lowercase() != "yes" || product.currency.to_lowercase() != "cad"
            {
                continue;
            }

            if product.total_variants > 0 {
                let _ = bigcommerce.enqueue_nested_product(
                    product.name.clone(),
                    product.image_url.clone(),
                    format!("{}?setCurrencyId=1", product.url),
                    search_term.category,
                );
                continue;
            }

            let mut price = Price {
                regular_price: price_to_cents(product.base_price.clone())?,
                sale_price: None,
            };

            if product.base_price != product.sale_price {
                price.sale_price = Some(price_to_cents(product.sale_price.clone())?);
            }

            let new_result = CrawlResult::new(
                product.name.clone(),
                product.url.clone(),
                price,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(product.image_url.clone());

            results.push(new_result);
        }

        results.extend(
            bigcommerce
                .parse_nested_products(SITE_URL, self.get_retailer_name())
                .await?,
        );

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms = Vec::from_iter([HtmlSearchQuery {
            term: "Shooting Supplies, Firearms & Ammunition;Firearms".into(),
            category: Category::Firearm,
        }]);

        let ammo_terms = [
            "Shooting Supplies, Firearms & Ammunition;Ammunition;Bulk Ammo",
            "Shooting Supplies, Firearms & Ammunition;Ammunition;Centerfire Ammunition",
            "Shooting Supplies, Firearms & Ammunition;Ammunition;Ammunition;Rimfire Ammunition",
            "Shooting Supplies, Firearms & Ammunition;Ammunition;Shotgun Ammunition",
        ];

        for ammo in ammo_terms {
            terms.push(HtmlSearchQuery {
                term: ammo.into(),
                category: Category::Ammunition,
            });
        }

        let other_terms = [
            "Shooting Supplies, Firearms & Ammunition;Stocks, Parts, Barrels & Kits",
            "Shooting Supplies, Firearms & Ammunition;Shooting Accessories",
            "Shooting Supplies, Firearms & Ammunition;Storage & Transportation",
            "Optics",
            "Shooting Supplies, Firearms & Ammunition;Ammunition;Reloading - Uncontrolled Items",
            "Shooting Supplies, Firearms & Ammunition;Ammunition;Reloading - Powders and Primers",
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
        let response = serde_json::from_str::<ApiResponse>(response)?;

        let Some(query_results) = response.query_results.first() else {
            return Ok(0);
        };

        Ok(query_results.meta.total_results_found / PAGE_LIMIT + 1)
    }
}
