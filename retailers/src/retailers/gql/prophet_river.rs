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
    let lowercase_path = path_node.to_ascii_lowercase();

    if lowercase_path.starts_with("/categories/rifles/")
        || lowercase_path.starts_with("/categories/shotguns/")
        || lowercase_path == "/shotguns/"
    {
        return CategoryMatch::Match(Category::Firearm);
    }

    match lowercase_path.as_str() {
        "/ammunition/" => CategoryMatch::Match(Category::Ammunition),
        "/reloading-equipment/"
        | "/reloading-components/"
        | "/rifle-scopes/"
        | "/optics-accessories/"
        | "/other-optics/"
        | "/stocks/"
        | "/accessories/" => CategoryMatch::Match(Category::Other),
        _ => CategoryMatch::Skip,
    }
}

const MAIN_URL: &str = "https://store.prophetriver.com";

pub struct ProphetRiver {
    auth_token: String,
}

impl Default for ProphetRiver {
    fn default() -> Self {
        Self::new()
    }
}

impl ProphetRiver {
    pub fn new() -> Self {
        Self {
            auth_token: String::new(),
        }
    }
}

impl GqlRetailerSuper for ProphetRiver {}

#[async_trait]
impl Retailer for ProphetRiver {
    async fn init(&mut self) -> Result<(), RetailerError> {
        self.auth_token = get_gql_token(MAIN_URL).await?;

        Ok(())
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::ProphetRiver
    }
}

#[async_trait]
impl GqlRetailer for ProphetRiver {
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
