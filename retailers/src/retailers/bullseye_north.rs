use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{ElementRef, Html, Selector};
use tracing::{debug, warn};

use crate::{
    errors::RetailerError,
    traits::{Retailer, SearchTerm},
    utils::{
        conversions::{price_to_cents, string_to_u64},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const PAGE_LIMIT: u64 = 36;
const URL: &str =
    "https://www.bullseyenorth.com/{category}/browse/perpage/{page_limit}/page/{page}";

pub struct BullseyeNorth {
    retailer: RetailerName,
}

impl BullseyeNorth {
    pub fn new() -> Self {
        Self {
            retailer: RetailerName::BullseyeNorth,
        }
    }

    fn get_price(product_element: ElementRef) -> Result<Price, RetailerError> {
        /*
        <span class="pricing">
            <strong class="itemPrice">$239.99</strong>
        </span>

        <span class="pricing">
            <strong class="listPrice">Regular Price: <span>$1,449.99</span></strong>
            <strong class="salePrice">$1,304.99</strong>
        </span> */

        let price_element = extract_element_from_element(product_element, "span.pricing")?;

        let mut price = Price {
            regular_price: 0,
            sale_price: None,
        };

        match extract_element_from_element(price_element, "strong.salePrice") {
            Ok(sale_element) => {
                let normal_price_element =
                    extract_element_from_element(price_element, "strong.listPrice > span")?;

                let normal_price = element_to_text(normal_price_element);

                price.regular_price = price_to_cents(normal_price)?;
                price.sale_price = Some(price_to_cents(element_to_text(sale_element))?);
            }
            Err(_) => {
                let normal_price_element =
                    extract_element_from_element(price_element, "strong.itemPrice")?;

                let normal_price = element_to_text(normal_price_element);

                price.regular_price = price_to_cents(normal_price)?;
            }
        };

        Ok(price)
    }
}

#[async_trait]
impl Retailer for BullseyeNorth {
    fn get_retailer_name(&self) -> RetailerName {
        self.retailer
    }

    async fn build_page_request(
        &self,
        page_num: u64,
        search_param: &SearchTerm,
    ) -> Result<Request, RetailerError> {
        let request = RequestBuilder::new()
            .set_url(
                URL.replace("{category}", &search_param.term)
                    .replace("{page_limit}", PAGE_LIMIT.to_string().as_str())
                    .replace("{page}", (page_num + 1).to_string().as_str()),
            )
            .build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_term: &SearchTerm,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let html = Html::parse_document(response);

        let product_selector = Selector::parse("a.product").unwrap();

        for product in html.select(&product_selector) {
            let name_element = extract_element_from_element(product, "span.name")?;
            let image_element = extract_element_from_element(product, "span.image > img")?;

            let url = element_extract_attr(product, "href")?;
            let name = element_to_text(name_element);
            let image = element_extract_attr(image_element, "src")?;

            if extract_element_from_element(product, "span.stock").is_err() {
                debug!("Skipping not in stock product {}", name);
                continue;
            }

            let price = Self::get_price(product)?;

            let new_result = CrawlResult::new(
                name,
                url,
                price,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(image.to_string());

            results.push(new_result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<SearchTerm> {
        Vec::from_iter([
            SearchTerm {
                term: "firearms".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "magazines".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "reloading".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "storage".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "optics".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories-cleaning-supplies".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories-eye-ear-protection".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories-gun-parts".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories-flashlights".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories-gunsmithing-tools".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories-holsters-magazine-pouches".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories-reference-manuals".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories-shooting-gear".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories-stock-bipod-sling".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories-targets".into(),
                category: Category::Other,
            },
        ])
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(response);

        let Ok(max_pages_el) = extract_element_from_element(html.root_element(), "p.paginTotals")
        else {
            warn!("Page missing total, probably no products in category");

            return Ok(0);
        };

        let max_page_count = element_extract_attr(max_pages_el, "data-max-pages")?;

        Ok(string_to_u64(max_page_count)?)
    }
}
