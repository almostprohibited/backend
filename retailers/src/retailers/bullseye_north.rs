use async_trait::async_trait;
use crawler::{
    request::{Request, RequestBuilder},
    unprotected::UnprotectedCrawler,
};
use scraper::{ElementRef, Html, Selector};
use tracing::error;

use crate::{
    errors::RetailerError,
    results::{
        constants::{ActionType, FirearmType, RetailerName},
        firearm::{FirearmPrice, FirearmResult},
    },
    traits::{Retailer, SearchParams},
    utils::{element_to_text, price_to_cents},
};

const PAGE_COOLDOWN: u64 = 5;
const PAGE_LIMIT: u8 = 36;
const URL: &str = "https://www.bullseyenorth.com/{category}/perpage/{page_limit}/page/{page}";

pub struct BullseyeNorth {
    crawler: UnprotectedCrawler,
}

impl BullseyeNorth {
    pub fn new() -> Self {
        Self {
            crawler: UnprotectedCrawler::new(),
        }
    }

    fn get_price(product_element: ElementRef) -> Result<FirearmPrice, RetailerError> {
        /*
        <span class="pricing">
            <strong class="itemPrice">$239.99</strong>
        </span>

        <span class="pricing">
            <strong class="listPrice">Regular Price: <span>$1,449.99</span></strong>
            <strong class="salePrice">$1,304.99</strong>
        </span> */

        let price_element_selector = Selector::parse("span.pricing").unwrap();
        let price_element = product_element
            .select(&price_element_selector)
            .next()
            .unwrap();

        let sale_selector = Selector::parse("strong.salePrice").unwrap();
        let normal_selector = Selector::parse("strong.itemPrice").unwrap();
        let normal_if_sale_selector = Selector::parse("strong.listPrice > span").unwrap();

        let mut price = FirearmPrice {
            regular_price: 0,
            sale_price: None,
        };

        match price_element.select(&sale_selector).next() {
            Some(sale_element) => {
                let normal_price_string = element_to_text(
                    price_element
                        .select(&normal_if_sale_selector)
                        .next()
                        .unwrap(),
                );
                price.regular_price = price_to_cents(normal_price_string)?;

                price.sale_price = Some(price_to_cents(element_to_text(sale_element))?);
            }
            None => {
                let normal_price_string =
                    element_to_text(price_element.select(&normal_selector).next().unwrap());

                price.regular_price = price_to_cents(normal_price_string)?;
            }
        };

        Ok(price)
    }
}

#[async_trait]
impl Retailer for BullseyeNorth {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_param: &SearchParams,
    ) -> Result<Request, RetailerError> {
        let request = RequestBuilder::new()
            .set_url(
                URL.replace("{category}", search_param.lookup)
                    .replace("{page_limit}", PAGE_LIMIT.to_string().as_str())
                    .replace("{page}", (page_num + 1).to_string().as_str()),
            )
            .build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_param: &SearchParams,
    ) -> Result<Vec<FirearmResult>, RetailerError> {
        let mut firearms: Vec<FirearmResult> = Vec::new();

        let html = Html::parse_document(response);

        let product_selector = Selector::parse("a.product").unwrap();

        for product in html.select(&product_selector) {
            let name_selector = Selector::parse("span.name").unwrap();
            let image_selector = Selector::parse("span.image > img").unwrap();

            let Some(url) = product.attr("href") else {
                error!("Product link is missing href");
                return Err(RetailerError::HtmlElementMissingAttribute(
                    "href".into(),
                    product.html(),
                ));
            };

            let name = element_to_text(product.select(&name_selector).next().unwrap());
            let image = product
                .select(&image_selector)
                .next()
                .unwrap()
                .attr("src")
                .unwrap();

            let price = Self::get_price(product);

            let mut new_firearm =
                FirearmResult::new(name, url, price?, RetailerName::BullseyeNorth);
            new_firearm.thumbnail_link = Some(image.to_string());
            new_firearm.action_type = search_param.action_type;
            new_firearm.ammo_type = search_param.ammo_type;
            new_firearm.firearm_class = search_param.firearm_class;
            new_firearm.firearm_type = search_param.firearm_type;

            firearms.push(new_firearm);
            break;
        }

        Ok(firearms)
    }

    fn get_search_parameters(&self) -> Result<Vec<SearchParams>, RetailerError> {
        let params = Vec::from_iter([
            // rifles
            SearchParams {
                lookup: "firearms-rifles/browse/guntype/bolt-action",
                action_type: Some(ActionType::BoltAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "firearms-rifles/browse/guntype/lever-action",
                action_type: Some(ActionType::LeverAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "firearms-rifles/browse/guntype/over%7Cunder",
                action_type: Some(ActionType::OverUnder),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "firearms-rifles/browse/guntype/pump-action",
                action_type: Some(ActionType::PumpAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "firearms-rifles/browse/guntype/semi-auto",
                action_type: Some(ActionType::SemiAuto),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "firearms-rifles/browse/guntype/single-shot",
                action_type: Some(ActionType::SingleShot),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            // shotguns
            SearchParams {
                lookup: "firearms-shotguns/browse/guntype/bolt-action",
                action_type: Some(ActionType::BoltAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "firearms-shotguns/browse/guntype/lever-action",
                action_type: Some(ActionType::LeverAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "firearms-shotguns/browse/guntype/over%7Cunder",
                action_type: Some(ActionType::OverUnder),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "firearms-shotguns/browse/guntype/pump-action",
                action_type: Some(ActionType::PumpAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "firearms-shotguns/browse/guntype/semi-auto",
                action_type: Some(ActionType::SemiAuto),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "firearms-shotguns/browse/guntype/single-shot",
                action_type: Some(ActionType::SingleShot),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
        ]);

        Ok(params)
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(response);
        let max_page_selector = Selector::parse("p.paginTotals").unwrap();

        let max_pages = html
            .select(&max_page_selector)
            .next()
            .unwrap()
            .attr("data-max-pages")
            .unwrap();

        Ok(max_pages.parse::<u64>().unwrap())
    }

    fn get_crawler(&self) -> UnprotectedCrawler {
        self.crawler
    }

    fn get_page_cooldown(&self) -> u64 {
        PAGE_COOLDOWN
    }
}
