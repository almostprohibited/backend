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
        gql_structs::ApiResponse,
    },
};

fn parse_category(path_node: &str) -> Option<Category> {
    if path_node == "/firearms/" {
        return Some(Category::Firearm);
    }

    // /reloading/ is duplicated a bunch here
    // since they also have powder listed, but they
    // don't ship powder
    if [
        "/cleaning-maintenance/",
        "/firearm-parts-and-upgrades/",
        "/flashlights-and-laser-combos/",
        "/holsters-mag-pouches-and-speed-belts/",
        "/optics-sights-and-mounts/",
        "/range-gear/",
        "/reloading-1/",
        "/range-supplies/",
        "/reloading/ammunition-boxes/",
        "/reloading/dies/",
        "/reloading/measuring-equipment/",
        "/reloading/prepping-equipment/",
        "/safety-personal-protection/",
        "/security-safes-and-locks/",
        "/targets/",
        "/tools/",
        "/training-systems/",
    ]
    .contains(&path_node)
        || path_node.starts_with("/firearm-parts/")
        || path_node.starts_with("/optics-sights-sight-mounts/")
        || path_node.starts_with("/reloading/reloading-presses/")
    {
        return Some(Category::Other);
    }

    None
}

const MAIN_URL: &str = "https://selectshootingsupplies.com";

pub struct SelectShootingSupplies {
    auth_token: String,
}

impl Default for SelectShootingSupplies {
    fn default() -> Self {
        Self::new()
    }
}

impl SelectShootingSupplies {
    pub fn new() -> Self {
        Self {
            auth_token: String::new(),
        }
    }
}

impl GqlRetailerSuper for SelectShootingSupplies {}

#[async_trait]
impl Retailer for SelectShootingSupplies {
    async fn init(&mut self) -> Result<(), RetailerError> {
        self.auth_token = get_gql_token(MAIN_URL).await?;

        Ok(())
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::SelectShootingSupplies
    }
}

#[async_trait]
impl GqlRetailer for SelectShootingSupplies {
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
