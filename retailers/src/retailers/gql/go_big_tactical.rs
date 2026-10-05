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

fn parse_category(path_node: &str) -> CategoryMatch {
    if path_node.to_lowercase().contains("/custom-rifle-builds/") {
        return CategoryMatch::Match(Category::Firearm);
    }

    CategoryMatch::Match(Category::Other)
}

const MAIN_URL: &str = "https://gobigtactical.ca";

pub struct GoBigTactical {
    auth_token: String,
}

impl Default for GoBigTactical {
    fn default() -> Self {
        Self::new()
    }
}

impl GoBigTactical {
    pub fn new() -> Self {
        Self {
            auth_token: String::new(),
        }
    }
}

impl GqlRetailerSuper for GoBigTactical {}

#[async_trait]
impl Retailer for GoBigTactical {
    async fn init(&mut self) -> Result<(), RetailerError> {
        self.auth_token = get_gql_token(MAIN_URL).await?;

        Ok(())
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::GoBigTactical
    }
}

#[async_trait]
impl GqlRetailer for GoBigTactical {
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

        response_objects.get_products(MAIN_URL, self.get_retailer_name(), parse_category)
    }

    fn get_pagination_token(&self, response: &str) -> Result<Option<String>, RetailerError> {
        let response_objects = serde_json::from_str::<ApiResponse>(response)?;

        Ok(response_objects.get_pagination_token())
    }
}
