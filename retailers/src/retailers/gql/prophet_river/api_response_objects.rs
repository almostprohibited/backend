use common::result::{base::Price, enums::Category};
use serde::Deserialize;

use crate::{errors::RetailerError, utils::conversions::price_to_cents};

#[derive(Deserialize, Debug)]
pub(super) struct ApiResponse {
    pub(super) data: ApiData,
}

#[derive(Deserialize, Debug)]
pub(super) struct ApiData {
    pub(super) site: ApiSite,
}

#[derive(Deserialize, Debug)]
pub(super) struct ApiSite {
    pub(super) products: ApiProducts,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(super) struct ApiProducts {
    pub(super) page_info: ApiPageInfo,
    pub(super) edges: Vec<ApiProductsEdge>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(super) struct ApiPageInfo {
    pub(super) end_cursor: Option<String>,
    pub(super) has_next_page: bool,
}

#[derive(Deserialize, Debug)]
pub(super) struct ApiProductsEdge {
    pub(super) node: ApiProductNode,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(super) struct ApiProductNode {
    pub(super) categories: ApiCategories,
    pub(super) name: String,
    pub(super) inventory: ApiInventory,
    pub(super) path: String,
    pub(super) default_image: Option<ApiImage>,
    pub(super) prices: ApiProductPrice,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(super) struct ApiProductPrice {
    pub(super) sale_price: Option<ApiPrice>,
    pub(super) price: ApiPrice,
}

impl ApiProductPrice {
    fn float_to_cents(original_price: f32) -> Result<u64, RetailerError> {
        Ok(price_to_cents(original_price.to_string())?)
    }

    pub(super) fn get_price(&self) -> Result<Price, RetailerError> {
        let mut price = Price {
            regular_price: Self::float_to_cents(self.price.value)?,
            sale_price: None,
        };

        if let Some(sale_price) = &self.sale_price {
            price.sale_price = Some(Self::float_to_cents(sale_price.value)?);
        }

        Ok(price)
    }
}

#[derive(Deserialize, Debug)]
pub(super) struct ApiPrice {
    pub(super) value: f32,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(super) struct ApiInventory {
    pub(super) is_in_stock: bool,
    pub(super) has_variant_inventory: bool,
}

#[derive(Deserialize, Debug)]
pub(super) struct ApiImage {
    pub(super) url: String,
}

#[derive(Deserialize, Debug)]
pub(super) struct ApiCategories {
    pub(super) edges: Vec<ApiCategoriesEdge>,
}

impl ApiCategories {
    pub(super) fn get_category(&self) -> Option<Category> {
        // so there should only be a single edge, but I can't
        // know for sure because the GQL response says that
        // it's a list, so I just have to belive the API
        for edge in &self.edges {
            let breadcrumbs = &edge.node.breadcrumbs.edges;

            for path_obj in breadcrumbs {
                let path_node = &path_obj.node;

                match path_node.path.clone().unwrap_or_default().as_str() {
                    "/categories/Rifles/" | "categories/Shotguns/" => {
                        return Some(Category::Firearm);
                    }
                    "/ammunition/" => return Some(Category::Ammunition),
                    "/reloading-equipment/"
                    | "/reloading-components/"
                    | "/rifle-scopes/"
                    | "/optics-accessories/"
                    | "/other-optics/"
                    | "/stocks/"
                    | "/accessories/" => {
                        return Some(Category::Other);
                    }
                    _ => {}
                }
            }
        }

        None
    }
}

#[derive(Deserialize, Debug)]
pub(super) struct ApiCategoriesEdge {
    pub(super) node: ApiCategoriesNode,
}

#[derive(Deserialize, Debug)]
pub(super) struct ApiCategoriesNode {
    pub(super) breadcrumbs: ApiCategoriesBreadcrumbs,
}

#[derive(Deserialize, Debug)]
pub(super) struct ApiCategoriesBreadcrumbs {
    pub(super) edges: Vec<ApiCategoriesBreadcrumbsEdge>,
}

#[derive(Deserialize, Debug)]
pub(super) struct ApiCategoriesBreadcrumbsEdge {
    pub(super) node: ApiCategoriesBreadcrumbsNode,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
pub(super) struct ApiCategoriesBreadcrumbsNode {
    // Un-used until I decide whether or not I want
    // string parsing, or ID parsing to determine
    // category type
    // pub(super) entity_id: u64,
    // pub(super) name: String,
    pub(super) path: Option<String>,
}
