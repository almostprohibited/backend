use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{ElementRef, Html, Selector};
use tracing::{debug, error};

use crate::{
    errors::RetailerError,
    traits::{Retailer, SearchTerm},
    utils::{
        conversions::price_to_cents,
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const URL: &str = "https://firearmsoutletcanada.com/{category}?in_stock=1&page={page}";

pub struct FirearmsOutletCanada {
    retailer: RetailerName,
}

impl FirearmsOutletCanada {
    pub fn new() -> Self {
        Self {
            retailer: RetailerName::FirearmsOutletCanada,
        }
    }

    fn create_price(element: ElementRef) -> Result<Price, RetailerError> {
        // <span data-product-non-sale-price-without-tax="" class="price price--non-sale"> $2,399.95 </span>
        // <span data-product-price-without-tax="" class="price price--withoutTax">$1,899.95</span>

        let main_price_el = extract_element_from_element(element, "span.price--withoutTax")?;
        let main_price = price_to_cents(element_to_text(main_price_el))?;

        let mut price = Price {
            regular_price: main_price,
            sale_price: None,
        };

        if let Ok(non_sale_price_el) = extract_element_from_element(element, "span.price--non-sale")
        {
            let non_sale_price = price_to_cents(element_to_text(non_sale_price_el))?;

            price.sale_price = Some(price.regular_price);
            price.regular_price = non_sale_price;
        };

        Ok(price)
    }
}

#[async_trait]
impl Retailer for FirearmsOutletCanada {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &SearchTerm,
    ) -> Result<Request, RetailerError> {
        let body = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", &(page_num + 1).to_string());

        let request = RequestBuilder::new().set_url(body).build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_term: &SearchTerm,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let html = Html::parse_document(response);

        let product_selector = Selector::parse("ul.productGrid > li.product").unwrap();

        for product in html.select(&product_selector) {
            let link_el = extract_element_from_element(product, "a.image-link.desktop")?;
            let img_name_el = extract_element_from_element(link_el, "img.primary")?;

            let link = element_extract_attr(link_el, "href")?;
            let name = element_extract_attr(img_name_el, "title")?;
            let image_link = element_extract_attr(img_name_el, "data-src")?;

            let price = Self::create_price(product)?;

            let new_result = CrawlResult::new(
                name,
                link,
                price,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(image_link);

            results.push(new_result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<SearchTerm> {
        let mut terms = Vec::from_iter([
            SearchTerm {
                term: "firearms".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "ammo".into(),
                category: Category::Ammunition,
            },
            // SearchTerm {
            //     term: "airguns".into(),
            //     category: Category::Firearm,
            // },
        ]);

        let other_terms = [
            "optics",
            "pistol-parts",
            "rifle-parts",
            "shotgun-parts",
            "magazines-clips",
            "reloading",
            "gear-kit",
            "storage-maintenance",
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
        let html = Html::parse_document(response);

        let dropdown_option_selector =
            Selector::parse("ul#facetedSearch-navList--bool > li > a").unwrap();

        let mut in_stock_element: Option<ElementRef> = None;

        for dropdown_option in html.select(&dropdown_option_selector) {
            let text = element_to_text(dropdown_option);

            debug!("text: {}", text);

            if text.contains("In Stock") {
                in_stock_element = Some(extract_element_from_element(dropdown_option, "span")?);
                break;
            }
        }

        let Some(unwrapped_in_stock_el) = in_stock_element else {
            return Ok(0);
        };

        let mut in_stock_text = element_to_text(unwrapped_in_stock_el);
        // the in stock text has () around it, like "(62)", use .replace()
        // because I don't want to deal with array slicing or regex
        in_stock_text = in_stock_text.replace("(", "").replace(")", "");

        let Ok(in_stock_count) = in_stock_text.parse::<u64>() else {
            let message = format!("Failed to parse {} into a number", in_stock_text);

            error!(message);

            return Err(RetailerError::GeneralError(message));
        };

        // for some reason, each page returns a max of exactly 52 items
        Ok(in_stock_count / 52)
    }

    fn get_retailer_name(&self) -> RetailerName {
        self.retailer
    }
}
