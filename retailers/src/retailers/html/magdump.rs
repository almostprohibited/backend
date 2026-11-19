use std::cmp::max;

use async_trait::async_trait;
use common::{
    constants::CRAWL_COOLDOWN_SECS,
    result::{
        base::{CrawlResult, Price},
        enums::{Category, RetailerName},
    },
};
use crawler::request::{Request, RequestBuilder};
use serde::Deserialize;
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::conversions::price_to_cents,
};

const URL: &str = "https://magdump.ca/{category}?from-xhr&page={page}";
const DELAY: u64 = 30;

#[derive(Deserialize)]
struct Response {
    products: Vec<ResponseProduct>,
    pagination: ResponsePagination,
}

impl Response {
    fn get_max_pages(&self) -> u64 {
        self.pagination.pages_count
    }
}

#[derive(Deserialize)]
struct ResponsePagination {
    pages_count: u64,
}

#[derive(Deserialize)]
struct ResponseProduct {
    add_to_cart_url: Option<String>,
    url: String,
    name: String,
    price_amount: f32,
    regular_price_amount: f32,
    cover: ResponseProductCover,
}

impl ResponseProduct {
    fn is_in_stock(&self) -> bool {
        self.add_to_cart_url.is_some()
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_url(&self) -> String {
        self.url.clone()
    }

    fn get_price(&self) -> Result<Price, RetailerError> {
        let regular = price_to_cents(self.regular_price_amount.to_string())?;
        let sale = price_to_cents(self.price_amount.to_string())?;

        Ok(Price {
            regular_price: regular,
            sale_price: if regular != sale { Some(sale) } else { None },
        })
    }

    fn get_image(&self) -> String {
        self.cover.small.url.clone()
    }
}

#[derive(Deserialize)]
struct ResponseProductCover {
    small: ResponseProductCoverImage,
}

#[derive(Deserialize)]
struct ResponseProductCoverImage {
    url: String,
}

pub struct MagDump;

impl Default for MagDump {
    fn default() -> Self {
        Self::new()
    }
}

impl MagDump {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for MagDump {}

#[async_trait]
impl Retailer for MagDump {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::MagDump
    }

    fn sleep_override(&self) -> Option<u64> {
        Some(max(DELAY, CRAWL_COOLDOWN_SECS))
    }
}

#[async_trait]
impl HtmlRetailer for MagDump {
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
            .set_headers(&[("Accept".to_string(), "application/json".to_string())])
            .build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let products = serde_json::from_str::<Response>(response)?;
        for product in products.products {
            if !product.is_in_stock() {
                continue;
            }

            let result = CrawlResult::new(
                product.get_name(),
                product.get_url(),
                product.get_price()?,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(product.get_image());

            results.push(result);
        }

        Ok(results)
    }

    // not the biggest fan of doing this but sitemap no longer lists categories
    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms = Vec::new();

        for firearm in [
            "46-rifles",
            // contains receivers, which they're not wrong to say it's a firearm
            // but just that these will likely be marked as "other" from us
            // since it's probably going to be double fetched in other
            "22-firearms",
        ] {
            terms.push(HtmlSearchQuery {
                term: firearm.into(),
                category: Category::Firearm,
            });
        }

        for firearm in ["11-rimfire", "12-centerfire", "41-bulk-amm"] {
            terms.push(HtmlSearchQuery {
                term: firearm.into(),
                category: Category::Ammunition,
            });
        }

        for other in [
            "30-accessories",
            "48-1022-barrels",
            "50-1022-bolts",
            "54-1022-magazine-adapter",
            "49-1022-parts",
            "47-1022-receivers",
            "55-1022-chassis-parts",
            "56-muzzle-brakes",
            "51-1022-chassis",
        ] {
            terms.push(HtmlSearchQuery {
                term: other.into(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let products = serde_json::from_str::<Response>(response)?;

        Ok(products.get_max_pages())
    }
}
