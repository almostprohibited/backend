use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::ecommerce::{Shopify, shopify::PAGE_LIMIT},
};

const URL: &str =
    "https://fishingworldgc.ca/collections/{category}/products.json?limit={page_limit}&page={page}";
const BASE_URL: &str = "https://fishingworldgc.ca";
const DEFAULT_IMAGE: &str =
    "https://intersurplus.com/cdn/shopifycloud/storefront/assets/no-image-50-e6fb86f4_360x.gif";

pub struct Fwgc;

impl Default for Fwgc {
    fn default() -> Self {
        Self::new()
    }
}

impl Fwgc {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for Fwgc {}

impl Retailer for Fwgc {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::Fwgc
    }
}

#[async_trait]
impl HtmlRetailer for Fwgc {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{page_limit}", &PAGE_LIMIT.to_string())
            .replace("{category}", &search_term.term.to_string())
            .replace("{page}", &(page_num + 1).to_string());

        debug!("Setting page to {}", url);

        let request = RequestBuilder::new().set_url(url).build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let helper = Shopify::new(DEFAULT_IMAGE, self.get_retailer_name(), BASE_URL);
        Ok(helper.parse_response(response, search_term)?)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut terms = vec![HtmlSearchQuery {
            term: "all-ammo-1".into(),
            category: Category::Ammunition,
        }];

        vec![
            "shot-gun",
            "centre-fire-rifle",
            "pre-owned",
            // annoyingly, they don't have a catch-all for rimfire
            // their rimfire "gun" category also contains ammo and clips
            "22wmr-1",
            "17hmr-2",
            "22lr-1",
        ]
        .iter()
        .for_each(|term| {
            terms.push(HtmlSearchQuery {
                term: term.to_string(),
                category: Category::Firearm,
            });
        });

        // they have sitemap, but contain too many categories for me to filter out
        // that I may as well just list them manually
        vec![
            "gun-parts",
            "packs-cases-1",
            "binoculars-1",
            "iron-sights-1",
            "range-finders-1",
            "red-dots",
            "scopes-1",
            "rings-mounts",
            "magazines-1",
            "gun-parts",
            "magpul-2",
            "shooting-miscellaneous-1",
            "stocks-grips-1",
            "slings-holsters-1",
            "targets",
            "locks-safes",
            "eye-ear-protection",
            "bullets-1",
            "primers",
            "powder",
            "reloading-supplies",
        ]
        .iter()
        .for_each(|term| {
            terms.push(HtmlSearchQuery {
                term: term.to_string(),
                category: Category::Other,
            });
        });

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        Shopify::get_pages(response)
    }
}
