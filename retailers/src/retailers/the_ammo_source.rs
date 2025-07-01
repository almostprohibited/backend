use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{ElementRef, Html, Selector};

use crate::{
    errors::RetailerError,
    traits::{Retailer, SearchTerm},
    utils::{
        conversions::{price_to_cents, string_to_u64},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
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

    fn get_price(element: ElementRef) -> Result<Price, RetailerError> {
        let main_price_element = extract_element_from_element(
            element,
            "div.price-section.price-section--withoutTax.current-price > span.price",
        )?;
        let main_price_text = element_to_text(main_price_element);

        let mut price = Price {
            regular_price: price_to_cents(main_price_text)?,
            sale_price: None,
        };

        if let Ok(non_sale_element) = extract_element_from_element(
            element,
            "div.price-section.price-section--withoutTax.non-sale-price > span.price",
        ) {
            price.sale_price = Some(price.regular_price);

            let non_sale_text = element_to_text(non_sale_element);
            price.regular_price = price_to_cents(non_sale_text)?;
        }

        Ok(price)
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
            let image_element =
                extract_element_from_element(product, "a.image-link.desktop > img.card-image")?;
            let image_url = element_extract_attr(image_element, "data-src")?;

            let details_body_element = extract_element_from_element(product, "div.card-body")?;
            let link_element =
                extract_element_from_element(details_body_element, "h4.card-title > a")?;

            let product_link = element_extract_attr(link_element, "href")?;
            let product_name = element_to_text(link_element);

            if product_name.contains("Sticker Draw") {
                continue;
            }

            let price = Self::get_price(details_body_element)?;

            let new_result = CrawlResult::new(
                product_name,
                product_link,
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
        Vec::from_iter([
            SearchTerm {
                term: "modern-sporting-rifles".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "rimfire-rifles".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "shotguns-hunting".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "shotguns-tactical".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "sporting-rifles".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "surplus-rifles-pistols".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "used-non-restricted-firearms".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "used-restricted-firearms".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "target-rifles".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "black-powder".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "air-guns".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "antique-firearms".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "oem-replacement-parts".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "scope-mounts-rings".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "scopes-optics-binos-and-sights".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "reloading-supplies".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "flashlights-batteries-illumination".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "firearms-accessories".into(),
                category: Category::Other,
            },
        ])
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(response);

        let selector =
            Selector::parse("li:not(.pagination-item--next).pagination-item > a.pagination-link")
                .unwrap();

        let pagination_elements = html.select(&selector);

        let Some(last_page_element) = pagination_elements.last() else {
            return Ok(0);
        };

        let last_page_text = element_to_text(last_page_element);

        Ok(string_to_u64(last_page_text)?)
    }

    fn get_retailer_name(&self) -> RetailerName {
        self.retailer
    }
}
