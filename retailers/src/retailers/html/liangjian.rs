use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use serde::Deserialize;
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::conversions::price_to_cents,
};

const MAX_ITEMS: u64 = 64;
const URL: &str = "https://31990017-c17c-4c86-89ca-5fc9b6a1bb06.mysimplestore.com/api/v2/products?page={page}&per_page={max_items}&taxon_permalink={category}";

#[derive(Deserialize, Debug)]
struct ApiResponse {
    pages: u64,
    products: Vec<ApiProduct>,
}

#[derive(Deserialize, Debug)]
struct ApiProduct {
    name: String,
    price: ApiPrice,
    sale_price: Option<ApiPrice>,
    default_asset_url: String,
    available: bool,
    in_stock: bool,
    relative_url: String,
}

#[derive(Deserialize, Debug, Clone)]
struct ApiPrice {
    display: String,
    currency: String,
}

impl ApiProduct {
    fn get_price(&self) -> Result<Price, RetailerError> {
        if self.price.currency.to_lowercase() != "cad" {
            return Err(RetailerError::ApiResponseInvalidShape(format!(
                "Regular price is not CAD, got: {}",
                self.price.currency
            )));
        }

        if let Some(sale_price) = self.sale_price.clone()
            && sale_price.currency.to_lowercase() != "cad"
        {
            return Err(RetailerError::ApiResponseInvalidShape(format!(
                "Sale price is not CAD, got: {}",
                self.price.currency
            )));
        }

        let price = Price {
            sale_price: self
                .sale_price
                .clone()
                .and_then(|sale_price| Some(price_to_cents(sale_price.display.clone()).ok()?)),
            regular_price: price_to_cents(self.price.display.clone())?,
        };

        Ok(price)
    }
}

pub struct Liangjian;

impl Default for Liangjian {
    fn default() -> Self {
        Self::new()
    }
}

impl Liangjian {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for Liangjian {}

impl Retailer for Liangjian {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::Liangjian
    }
}

#[async_trait]
impl HtmlRetailer for Liangjian {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{max_items}", &MAX_ITEMS.to_string())
            .replace("{page}", (page_num + 1).to_string().as_str());

        debug!("Setting page to {}", url);

        let request = RequestBuilder::new().set_url(url).build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let parsed_response = serde_json::from_str::<ApiResponse>(response)?;

        let mut results: Vec<CrawlResult> = Vec::new();

        for product in parsed_response.products {
            if !(product.available && product.in_stock) {
                debug!("Skipping due to not available: {product:?}");

                continue;
            }

            let new_result = CrawlResult::new(
                product.name.clone(),
                format!("https://liangjian.ca/shop{}", product.relative_url),
                product.get_price()?,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(product.default_asset_url.clone());

            results.push(new_result);
        }

        Ok(results)
    }

    // TODO: replace this with sitemap parsing, didn't do it at time of
    // creating this since their site is constantly 503 when visiting
    // https://liangjian.ca/sitemap.ols.xml
    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms = Vec::from_iter([
            HtmlSearchQuery {
                term: "rifle".into(),
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "shotguns".into(),
                category: Category::Firearm,
            },
            HtmlSearchQuery {
                term: "ammunition".into(),
                category: Category::Ammunition,
            },
        ]);

        for other in [
            "optics",
            "magazine",
            "reloading-stuff",
            "accessory",
            "scope-rings-and-mounts",
            "magpul",
            "mdt",
            "sj-hardware",
            "spuhr",
            "midwest-industries",
            "parts",
            "body-armors-helmets",
            "self-defense",
            "crossbows",
            "knives",
            "byrna",
            "militaria",
            "snap-caps",
            "tactical-gears",
            "tools",
            "emergency-supply",
        ] {
            terms.push(HtmlSearchQuery {
                term: other.into(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let parsed_response = serde_json::from_str::<ApiResponse>(response)?;

        Ok(parsed_response.pages)
    }
}
