use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{ElementRef, Html, Selector};
use tracing::debug;

use crate::{
    errors::RetailerError,
    traits::{Retailer, SearchTerm},
    utils::{
        conversions::price_to_cents,
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const ITEMS_PER_PAGE: u64 = 25;
const URL: &str = "https://www.italiansportinggoods.com/{category}.html?product_list_limit={items_per_page}&p={page}";

pub struct ItalianSportingGoods {
    retailer: RetailerName,
}

impl ItalianSportingGoods {
    pub fn new() -> Self {
        Self {
            retailer: RetailerName::ItalianSportingGoods,
        }
    }

    fn parse_prices(element: ElementRef) -> Result<Price, RetailerError> {
        let final_price_el = extract_element_from_element(
            element,
            "span.price-wrapper[data-price-type=finalPrice] > span",
        )?;
        let final_price = price_to_cents(element_to_text(final_price_el))?;

        let mut price = Price {
            regular_price: final_price,
            sale_price: None,
        };

        if let Ok(old_price_element) = extract_element_from_element(
            element,
            "span.price-wrapper[data-price-type=oldPrice] > span",
        ) {
            price.regular_price = price_to_cents(element_to_text(old_price_element))?;
            price.sale_price = Some(final_price);
        };

        Ok(price)
    }
}

#[async_trait]
impl Retailer for ItalianSportingGoods {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &SearchTerm,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", &(page_num + 1).to_string())
            .replace("{items_per_page}", &ITEMS_PER_PAGE.to_string());

        debug!("Setting page to {}", url);

        Ok(RequestBuilder::new().set_url(url).build())
    }

    async fn parse_response(
        &self,
        response: &String,
        search_term: &SearchTerm,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let html = Html::parse_document(&response);

        let product_selector = Selector::parse("div.product-item-info").unwrap();

        for element in html.select(&product_selector) {
            let details_element =
                extract_element_from_element(element, "div.product-item-details")?;
            let link_element =
                extract_element_from_element(details_element, "a.product-item-link")?;

            if let Ok(data_bind_attr) = element_extract_attr(link_element, "data-bind")
                && !data_bind_attr.is_empty()
            {
                continue;
            }

            let url = element_extract_attr(link_element, "href")?;
            let name = element_to_text(link_element);
            let price = Self::parse_prices(details_element)?;

            let image_element = extract_element_from_element(
                element,
                "a.product-item-photo img.product-image-photo",
            )?;
            let image_url = element_extract_attr(image_element, "src")?;

            let new_result = CrawlResult::new(
                name,
                url,
                price,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(image_url);

            results.push(new_result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<SearchTerm> {
        let mut terms = Vec::from_iter([SearchTerm {
            term: "firearms".into(),
            category: Category::Firearm,
        }]);

        let other_terms = [
            "optics",
            "reloading",
            "shooting",
            "ar-accessories",
            "gun-care",
            "gun-cases-and-storage",
        ];

        for other in other_terms {
            terms.push(SearchTerm {
                term: other.into(),
                category: Category::Other,
            });
        }

        terms
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(&response);

        let item_counts = Selector::parse("p#toolbar-amount > span.toolbar-number").unwrap();
        if let Some(total_items_element) = html.select(&item_counts).nth(2) {
            let count = element_to_text(total_items_element)
                .parse::<f32>()
                .expect("Count not a number");

            Ok((count / ITEMS_PER_PAGE as f32).ceil() as u64)
        } else {
            Ok(0)
        }
    }

    fn get_retailer_name(&self) -> RetailerName {
        self.retailer
    }
}
