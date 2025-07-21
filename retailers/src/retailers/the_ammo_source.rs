use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{ElementRef, Html, Selector};

use crate::{
    errors::RetailerError,
    traits::{Retailer, SearchTerm},
    utils::{
        ecommerce::bigcommerce::BigCommerce,
        html::{element_to_text, extract_element_from_element},
    },
};

const URL: &str = "https://theammosource.com/{category}/?page={page}&in_stock=1";

pub struct TheAmmoSource {
    retailer: RetailerName,
}

impl TheAmmoSource {
    pub fn new() -> Self {
        Self {
            retailer: RetailerName::TheAmmoSource,
        }
    }

    // SFRC has these sticker/lottery draws that aren't
    // products and shouldn't be included
    fn is_sticker_draw(element: ElementRef) -> Result<bool, RetailerError> {
        let details_body_element = extract_element_from_element(element, "div.card-body")?;
        let link_element = extract_element_from_element(details_body_element, "h4.card-title > a")?;

        let product_name = element_to_text(link_element);

        Ok(product_name.contains("Sticker Draw"))
    }
}

#[async_trait]
impl Retailer for TheAmmoSource {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &SearchTerm,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", &(page_num + 1).to_string());

        let request = RequestBuilder::new().set_url(url).build();

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
            if Self::is_sticker_draw(product)? {
                continue;
            }

            let result = BigCommerce::parse_product(
                product,
                self.get_retailer_name(),
                search_term.category,
            )?;

            results.push(result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<SearchTerm> {
        let mut terms = Vec::from_iter([SearchTerm {
            term: "ammunition".into(),
            category: Category::Ammunition,
        }]);

        let firearm_terms = [
            "modern-sporting-rifles",
            "rimfire-rifles",
            "shotguns-hunting",
            "shotguns-tactical",
            "sporting-rifles",
            "surplus-rifles-pistols",
            "used-non-restricted-firearms",
            "used-restricted-firearms",
            "target-rifles",
            "black-powder",
            "air-guns",
            "antique-firearms",
        ];

        for firearm in firearm_terms {
            terms.push(SearchTerm {
                term: firearm.into(),
                category: Category::Firearm,
            });
        }

        let other_terms = [
            "oem-replacement-parts",
            "scope-mounts-rings",
            "scopes-optics-binos-and-sights",
            "reloading-supplies",
            "flashlights-batteries-illumination",
            "firearms-accessories",
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
        BigCommerce::parse_max_pages(response)
    }

    fn get_retailer_name(&self) -> RetailerName {
        self.retailer
    }
}
