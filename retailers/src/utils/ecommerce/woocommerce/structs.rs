use std::collections::HashMap;

use common::result::enums::Category;
use serde::Deserialize;

pub(super) struct NestedProduct {
    pub(super) url: String,
    pub(super) category: Category,
}

#[derive(Deserialize, Debug)]
pub(super) struct ProductImage {
    pub(super) url: String,
}

#[derive(Deserialize, Debug)]
pub(super) struct ProductVariation {
    pub(super) attributes: HashMap<String, String>,
    pub(super) image: ProductImage,
    pub(super) is_in_stock: bool,
    pub(super) display_price: f32,
    pub(super) display_regular_price: f32,
}
