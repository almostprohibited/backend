use common::result::enums::Category;
use serde::Deserialize;

#[derive(Deserialize)]
pub(crate) struct NestedApiResponse {
    pub(crate) data: NestedApiResponseData,
}

#[derive(Deserialize)]
pub(crate) struct NestedApiResponseData {
    pub(crate) image: Option<NestedApiResponseImage>,
    pub(crate) instock: bool,
    pub(crate) price: NestedApiResponsePrice,
}

#[derive(Deserialize)]
pub(crate) struct NestedApiResponseImage {
    pub(crate) data: String,
}

impl NestedApiResponseImage {
    pub(crate) fn get_image(&self) -> String {
        self.data.replace(REPLACEMENT_PATTERN, REPLACEMENT_SIZE)
    }
}

#[derive(Deserialize)]
pub(crate) struct NestedApiResponsePrice {
    pub(crate) without_tax: NestedApiPrice,
    pub(crate) non_sale_price_without_tax: Option<NestedApiPrice>,
}

#[derive(Deserialize)]
pub(crate) struct NestedApiPrice {
    pub(crate) value: f32,
    pub(crate) currency: String,
}

const REPLACEMENT_PATTERN: &str = "{:size}";
const REPLACEMENT_SIZE: &str = "300w";

#[derive(Debug, Clone)]
pub(crate) struct FormValuePair {
    pub(crate) form_id: String,
    pub(crate) form_attr_id: String,
    pub(crate) attr_name: String,
}

#[derive(Debug)]
pub(crate) struct QueryParams {
    pub(crate) form_pairs: Vec<Vec<FormValuePair>>,
}

impl QueryParams {
    pub(crate) fn new() -> Self {
        Self {
            form_pairs: Vec::new(),
        }
    }

    pub(crate) fn apply(&mut self, form_pairs: Vec<FormValuePair>) {
        if self.form_pairs.is_empty() {
            for pair in form_pairs {
                let new_vec: Vec<FormValuePair> = vec![pair];

                self.form_pairs.push(new_vec);
            }
        } else {
            for new_pair in form_pairs {
                for current_pairs in &mut self.form_pairs {
                    current_pairs.push(new_pair.clone());
                }
            }
        }
    }
}

pub(crate) struct NestedProduct {
    pub(crate) name: String,
    pub(crate) fallback_image_url: String,
    pub(crate) category: Category,
    pub(crate) product_url: String,
}

#[derive(Deserialize)]
pub(super) struct JavascriptJson {
    pub(super) product_attributes: JavascriptJsonProducts,
}

#[derive(Deserialize)]
pub(super) struct JavascriptJsonProducts {
    pub(super) in_stock_attributes: Vec<u64>,
}

pub(crate) struct SitemapEntry {
    pub(crate) name: String,
    pub(crate) part: String,
}
