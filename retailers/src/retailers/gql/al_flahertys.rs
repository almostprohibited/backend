use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::Request;

use crate::{
    errors::RetailerError,
    structures::{GqlRetailer, GqlRetailerSuper, Retailer},
    utils::ecommerce::bigcommerce::{
        gql_helpers::{build_request, get_gql_token},
        gql_structs::{ApiResponse, CategoryMatch},
    },
};

const MAIN_URL: &str = "https://alflahertys.com/";

// ordering in this method is important
// there are products that belong under same parent
// that are not actually related for what we need
fn product_classifier(path_node: &str) -> CategoryMatch {
    if path_node.starts_with("/optics/")
        | path_node.starts_with(
            "/shooting-supplies-and-firearms/ammunition/reloading-powders-and-primers/",
        )
        | path_node
            .starts_with("/shooting-supplies-and-firearms/stocks-parts-barrels-conversion-kits/")
        | path_node.starts_with("/shooting-supplies-and-firearms/storage-transportation/")
        | path_node.starts_with("/shooting-supplies-and-firearms/tactical-accessories/")
        | path_node.starts_with(
            "/shooting-supplies-firearms-ammunition/ammunition/reloading---uncontrolled-items/",
        )
        | path_node
            .starts_with("/shooting-supplies-firearms-and-ammunition/stocks-parts-barrels-kits/")
    {
        return CategoryMatch::Match(Category::Other);
    }

    if path_node == "/shooting-supplies-firearms-and-ammunition/firearms/"
        || path_node.starts_with("/shooting-supplies-firearms-ammunition/firearms/")
    {
        return CategoryMatch::Match(Category::Firearm);
    }

    if path_node == "/shooting-supplies-and-firearms/ammunition/bulk-centerfire/"
        || path_node.starts_with("/shooting-supplies-firearms-ammunition/ammunition/")
    {
        return CategoryMatch::Match(Category::Ammunition);
    }

    // this is here on purpose, ammo and other stuff gets matched above
    // this will hopefully catch everything else
    if path_node.starts_with("/shooting-supplies-firearms-ammunition/") {
        return CategoryMatch::Match(Category::Other);
    }

    return CategoryMatch::Skip;
}

pub struct AlFlahertys {
    auth_token: String,
}

impl Default for AlFlahertys {
    fn default() -> Self {
        Self::new()
    }
}

impl AlFlahertys {
    pub fn new() -> Self {
        Self {
            auth_token: String::new(),
        }
    }
}

impl GqlRetailerSuper for AlFlahertys {}

#[async_trait]
impl Retailer for AlFlahertys {
    async fn init(&mut self) -> Result<(), RetailerError> {
        self.auth_token = get_gql_token(MAIN_URL).await?;

        Ok(())
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::AlFlahertys
    }
}

#[async_trait]
impl GqlRetailer for AlFlahertys {
    async fn build_page_request(
        &self,
        pagination_token: Option<String>,
    ) -> Result<Request, RetailerError> {
        let mut gql_url = MAIN_URL.to_string();

        if !gql_url.ends_with("/") {
            gql_url.push('/');
        }

        build_request(
            &format!("{gql_url}graphql"),
            &self.auth_token,
            pagination_token,
        )
    }

    async fn parse_response(&self, response: &str) -> Result<Vec<CrawlResult>, RetailerError> {
        let response_objects = serde_json::from_str::<ApiResponse>(response)?;

        response_objects.get_products(MAIN_URL, self.get_retailer_name(), product_classifier)
    }

    fn get_pagination_token(&self, response: &str) -> Result<Option<String>, RetailerError> {
        let response_objects = serde_json::from_str::<ApiResponse>(response)?;

        Ok(response_objects.get_pagination_token())
    }
}
