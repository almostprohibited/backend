use serde::Deserialize;
use serde_json::{Value, json};

/// Arugment is named correctly, API says product ID but actual value is template ID
pub(super) fn get_quick_view_api_body(template_id: u64) -> Value {
    json!({
        "id": 0,
        "jsonrpc": "2.0",
        "method": "call",
        "params": {
            "options": {
                "productID": template_id,
                "variantID": false,
                "variant_selector": false
            }
        }
    })
}

pub(super) fn get_variant_api_body(
    variant_array: Vec<u64>,
    product_id: u64,
    template_id: u64,
) -> Value {
    json!({
        "id": 0,
        "jsonrpc": "2.0",
        "method": "call",
        "params": {
            "add_qty": 1,
            "combination": variant_array,
            "parent_combination": [],
            "product_id": product_id,
            "product_template_id": template_id
        }
    })
}

#[derive(Deserialize)]
pub(super) struct QuickViewResponse {
    pub(super) result: String,
}

#[derive(Deserialize)]
pub(super) struct VariantResponse {
    pub(super) result: VariantResponseResult,
}

#[derive(Deserialize)]
pub(super) struct VariantResponseResult {
    pub(super) display_name: String,
    pub(super) is_combination_possible: bool,
    pub(super) free_qty: f32,
    /// Base price, 0.0 if no sale
    pub(super) compare_list_price: f32,
    /// Sale price, or base price if above is 0.0
    pub(super) list_price: f32,
}
