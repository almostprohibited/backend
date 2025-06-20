use async_trait::async_trait;
use common::firearms::{
    constants::{ActionType, FirearmClass, FirearmType, RetailerName},
    firearm::{Firearm, FirearmPrice},
};
use crawler::{
    request::{Request, RequestBuilder},
    unprotected::UnprotectedCrawler,
};
use scraper::{Html, Selector};
use tracing::debug;

use crate::{
    errors::RetailerError,
    traits::{Retailer, SearchParams},
    utils::{
        conversions::price_to_cents,
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const PAGE_COOLDOWN: u64 = 10;
const URL: &str = "https://www.canadasgunstore.ca/inet/storefront/store.php?mode=browsecategory&department=30&class=FA&fineline={type}&attr[A2]={action}&refine=Y";

pub struct CanadasGunStore {
    crawler: UnprotectedCrawler,
    retailer: RetailerName,
}

impl CanadasGunStore {
    pub fn new() -> Self {
        Self {
            crawler: UnprotectedCrawler::new(),
            retailer: RetailerName::CanadasGunStore,
        }
    }
}

#[async_trait]
impl Retailer for CanadasGunStore {
    fn get_retailer_name(&self) -> RetailerName {
        self.retailer
    }

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
    ) -> Result<Vec<Firearm>, RetailerError> {
        let mut firearms: Vec<Firearm> = Vec::new();

        let html = Html::parse_document(response);

        let product_selector = Selector::parse("div.product_body").unwrap();

        for product in html.select(&product_selector) {
            let stock_element =
                extract_element_from_element(product, "span.product_status".into())?;

            if element_to_text(stock_element) != "In Stock" {
                debug!("Skipping out of stock item");
                continue;
            }

            let name_link_element =
                extract_element_from_element(product, "h4.store_product_name > a".into())?;

            let image_element = extract_element_from_element(product, "img.product_image".into())?;

            let price_element = extract_element_from_element(product, "div.product_price".into())?;

            let url = format!(
                "https://www.canadasgunstore.ca{}",
                element_extract_attr(name_link_element, "href".into())?
            );

            let name = element_to_text(name_link_element);
            let image = element_extract_attr(image_element, "src".into())?;

            let price = price_to_cents(element_to_text(price_element))?;

            let firearm_price = FirearmPrice {
                regular_price: price,
                sale_price: None,
            };

            let mut new_firearm = Firearm::new(name, url, firearm_price, self.get_retailer_name());
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
