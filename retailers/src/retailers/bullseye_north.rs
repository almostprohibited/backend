use async_trait::async_trait;
use crawler::{
    request::{Request, RequestBuilder},
    unprotected::UnprotectedCrawler,
};
use scraper::{ElementRef, Html, Selector};
use tracing::warn;

use crate::{
    errors::RetailerError,
    results::{
        constants::{ActionType, FirearmType, RetailerName},
        firearm::{FirearmPrice, FirearmResult},
    },
    traits::{Retailer, SearchParams},
    utils::{
        conversions::{price_to_cents, string_to_u64},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const PAGE_COOLDOWN: u64 = 5;
const PAGE_LIMIT: u64 = 36;
const URL: &str = "https://www.bullseyenorth.com/{category}/perpage/{page_limit}/page/{page}";

pub struct BullseyeNorth {
    crawler: UnprotectedCrawler,
    retailer: RetailerName,
}

impl BullseyeNorth {
    pub fn new() -> Self {
        Self {
            crawler: UnprotectedCrawler::new(),
            retailer: RetailerName::BullseyeNorth,
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

        let price_element = extract_element_from_element(product_element, "span.pricing".into())?;

        let mut price = FirearmPrice {
            regular_price: 0,
            sale_price: None,
        };

        match extract_element_from_element(price_element, "strong.salePrice".into()) {
            Ok(sale_element) => {
                let normal_price_element =
                    extract_element_from_element(price_element, "strong.listPrice > span".into())?;

                let normal_price = element_to_text(normal_price_element);

                price.regular_price = price_to_cents(normal_price)?;
                price.sale_price = Some(price_to_cents(element_to_text(sale_element))?);
            }
            Err(_) => {
                let normal_price_element =
                    extract_element_from_element(price_element, "strong.itemPrice".into())?;

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
            let name_element = extract_element_from_element(product, "span.name".into())?;
            let image_element = extract_element_from_element(product, "span.image > img".into())?;

            let url = element_extract_attr(product, "href".into())?;
            let name = element_to_text(name_element);
            let image = element_extract_attr(image_element, "src".into())?;

            let price = Self::get_price(product)?;

            let mut new_firearm = FirearmResult::new(name, url, price, self.get_retailer_name());
            new_firearm.thumbnail_link = Some(image.to_string());
            new_firearm.action_type = search_param.action_type;
            new_firearm.ammo_type = search_param.ammo_type;
            new_firearm.firearm_class = search_param.firearm_class;
            new_firearm.firearm_type = search_param.firearm_type;

            firearms.push(new_firearm);
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

        let Ok(max_pages_el) =
            extract_element_from_element(html.root_element(), "p.paginTotals".into())
        else {
            warn!("Page missing total, probably no products in category");

            return Ok(0);
        };

        let max_page_count = element_extract_attr(max_pages_el, "data-max-pages".into())?;

        let item_as_int = string_to_u64(max_page_count)?;

        Ok((item_as_int / PAGE_LIMIT).into())
    }

    fn get_crawler(&self) -> UnprotectedCrawler {
        self.crawler
    }

    fn get_page_cooldown(&self) -> u64 {
        PAGE_COOLDOWN
    }
}
