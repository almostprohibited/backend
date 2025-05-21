use async_trait::async_trait;
use crawler::{
    request::{Request, RequestBuilder},
    unprotected::UnprotectedCrawler,
};
use scraper::{ElementRef, Html, Selector};
use tracing::debug;

use crate::{
    errors::RetailerError,
    results::{
        constants::{ActionType, FirearmClass, FirearmType, RetailerName},
        firearm::{FirearmPrice, FirearmResult},
    },
    traits::{Retailer, SearchParams},
    utils::{element_to_text, price_to_cents},
};

const PAGE_COOLDOWN: u64 = 10;
const URL: &str = "https://www.canadasgunstore.ca/inet/storefront/store.php?mode=browsecategory&department=30&class=FA&fineline={type}&attr[A2]={action}&refine=Y";

pub struct CanadasGunStore {
    crawler: UnprotectedCrawler,
}

impl CanadasGunStore {
    pub fn new() -> Self {
        Self {
            crawler: UnprotectedCrawler::new(),
        }
    }
}

#[async_trait]
impl Retailer for CanadasGunStore {
    async fn build_page_request(
        &self,
        _page_num: u64,
        search_param: &SearchParams,
    ) -> Result<Request, RetailerError> {
        let firearm_type = match search_param.firearm_type.unwrap() {
            FirearmType::Rifle => "RIFLNR",
            FirearmType::Shotgun => "SHOTGN",
        };

        let action_type = match search_param.action_type.unwrap() {
            ActionType::SemiAuto => "SEMIAUTO",
            ActionType::LeverAction => "LEVER",
            ActionType::BreakAction => "BREAK",
            ActionType::BoltAction => "BOLTACTION",
            ActionType::OverUnder => "OVER%2FUNDER",
            ActionType::PumpAction => "PUMP",
            ActionType::SideBySide => "SIDEBYSIDE",
            ActionType::SingleShot => "SINGLE",
            ActionType::Revolver => "REVOLVER",
            ActionType::StraightPull => "STRIAGHT",
        };

        let request = RequestBuilder::new()
            .set_url(
                URL.replace("{type}", firearm_type)
                    .replace("{action}", action_type),
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

        let product_selector = Selector::parse("div.product_body").unwrap();

        for product in html.select(&product_selector) {
            let in_stock_selector = Selector::parse("span.product_status").unwrap();
            let stock_element = product.select(&in_stock_selector).next().unwrap();

            if element_to_text(stock_element) != "In Stock" {
                debug!("Skipping out of stock item");
                continue;
            }

            let name_link_selector = Selector::parse("h4.store_product_name > a").unwrap();
            let image_selector = Selector::parse("img.product_image").unwrap();
            let price_selector = Selector::parse("div.product_price").unwrap();

            let name_link_element: ElementRef<'_> =
                product.select(&name_link_selector).next().unwrap();

            let url = format!(
                "https://www.canadasgunstore.ca{}",
                name_link_element.attr("href").unwrap()
            );

            let name = element_to_text(name_link_element);
            let image = product
                .select(&image_selector)
                .next()
                .unwrap()
                .attr("src")
                .unwrap();

            let price = price_to_cents(element_to_text(
                product.select(&price_selector).next().unwrap(),
            ))?;

            let firearm_price = FirearmPrice {
                regular_price: price,
                sale_price: None,
            };

            let mut new_firearm =
                FirearmResult::new(name, url, firearm_price, RetailerName::CanadasGunStore);
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
                lookup: "",
                action_type: Some(ActionType::LeverAction),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::BoltAction),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::SemiAuto),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::PumpAction),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::BreakAction),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::OverUnder),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::SideBySide),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::SingleShot),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::Revolver),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::StraightPull),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Rifle),
            },
            // shotguns
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::LeverAction),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::BoltAction),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::SemiAuto),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::PumpAction),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::BreakAction),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::OverUnder),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::SideBySide),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::SingleShot),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::Revolver),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::StraightPull),
                ammo_type: None,
                firearm_class: Some(FirearmClass::NonRestricted),
                firearm_type: Some(FirearmType::Shotgun),
            },
        ]);

        Ok(params)
    }

    fn get_num_pages(&self, _response: &String) -> Result<u64, RetailerError> {
        Ok(0)
    }

    fn get_crawler(&self) -> UnprotectedCrawler {
        self.crawler
    }

    fn get_page_cooldown(&self) -> u64 {
        PAGE_COOLDOWN
    }
}
