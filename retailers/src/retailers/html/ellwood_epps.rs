// TODO: refactor this retailer into an "API" specific one as this does not do normal HTML things

use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use csv::Reader;
use serde::Deserialize;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::conversions::price_to_cents,
};

const PRODUCT_CSV: &str = "https://ellwoodepps.com/products.csv";

#[derive(Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
struct EllwoodEppsProductData {
    make: String,
    model: String,
    category: String,
    calibre: String,
    /// Blank string if no sale price, populated with original price if there is a sale price
    original_price: String,
    /// Contains current pricing, if `original_price` is populated then `price` is sale price
    price: String,
    quantity_available: u64,
    url: String,
    image_url: String,
}

impl EllwoodEppsProductData {
    fn get_category_mapping(&self) -> Category {
        match self.category.to_lowercase().as_str() {
            "combination gun" | "shotgun" | "rifle" | "handgun" => Category::Firearm,
            "ammo" => Category::Ammunition,
            // yolo, map everything else into other
            _ => Category::Other,
        }
    }

    fn is_valid_product(&self) -> bool {
        self.quantity_available > 0
    }

    fn get_name(&self) -> String {
        let base_name = format!("{} {}", self.make, self.model);

        if !self.calibre.is_empty() {
            return format!("{base_name} - {}", self.calibre);
        }

        base_name
    }

    fn get_url(&self) -> String {
        self.url.replace("img.ellwoodepps.com", "ellwoodepps.com")
    }

    fn get_price(&self) -> Result<Price, RetailerError> {
        let listed_price = price_to_cents(self.price.clone())?;

        let mut price = Price {
            regular_price: listed_price,
            sale_price: None,
        };

        if !self.original_price.is_empty() {
            let original_price = price_to_cents(self.original_price.clone())?;

            price.sale_price = Some(price.regular_price);
            price.regular_price = original_price;
        }

        Ok(price)
    }
}

impl TryInto<CrawlResult> for EllwoodEppsProductData {
    type Error = RetailerError;

    fn try_into(self) -> Result<CrawlResult, Self::Error> {
        let result = CrawlResult::new(
            self.get_name(),
            self.get_url(),
            self.get_price()?,
            RetailerName::EllwoodEpps,
            self.get_category_mapping(),
        )
        .with_image_url(self.image_url);

        Ok(result)
    }
}

pub struct EllwoodEpps;

impl Default for EllwoodEpps {
    fn default() -> Self {
        Self::new()
    }
}

impl EllwoodEpps {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for EllwoodEpps {}

impl Retailer for EllwoodEpps {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::EllwoodEpps
    }
}

#[async_trait]
impl HtmlRetailer for EllwoodEpps {
    async fn build_page_request(
        &self,
        _page_num: u64,
        _search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let request = RequestBuilder::new().set_url(PRODUCT_CSV).build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        _search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let mut reader = Reader::from_reader(response.as_str().as_bytes());
        for row in reader.deserialize() {
            let record: EllwoodEppsProductData = row.map_err(|err| {
                RetailerError::GeneralError(format!("Failed to map CSV row: {err}"))
            })?;

            if !record.is_valid_product() {
                continue;
            }

            let mut result: CrawlResult = record.try_into()?;

            if result.category == Category::Ammunition
                && result.name.to_lowercase().starts_with("hodgdon")
            {
                result.update_category(Category::Other);
            }

            results.push(result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        vec![HtmlSearchQuery {
            term: String::default(),
            category: Category::Other,
        }]
    }

    fn get_num_pages(&self, _response: &String) -> Result<u64, RetailerError> {
        Ok(0)
    }
}
