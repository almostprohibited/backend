use async_trait::async_trait;
use crawler::{
    request::{Request, RequestBuilder},
    unprotected::UnprotectedCrawler,
};
use regex::Regex;
use scraper::{ElementRef, Html, Selector};
use tracing::debug;

use crate::{
    errors::RetailerError,
    results::{
        constants::{ActionType, FirearmType, RetailerName},
        firearm::{FirearmPrice, FirearmResult},
    },
    traits::{Retailer, SearchParams},
    utils::{element_to_text, price_to_cents},
};

const PAGE_COOLDOWN: u64 = 10;
const PAGE_LIMIT: u64 = 100;
const URL: &str = "https://store.theshootingcentre.com/firearms/{firearm_type}/?limit={page_limit}&mode=6&Action+Type={action}&page={page}";

pub struct CanadasGunShop {
    crawler: UnprotectedCrawler,
}

impl CanadasGunShop {
    pub fn new() -> Self {
        Self {
            crawler: UnprotectedCrawler::new(),
        }
    }

    fn get_price(product_element: ElementRef) -> Result<FirearmPrice, RetailerError> {
        /*
        <span data-product-price-without-tax="" class="price price--withoutTax price--main">$2,160.00</span>
        <span data-product-non-sale-price-without-tax="" class="price price--non-sale">$2,400.00</span>

        <span data-product-non-sale-price-without-tax="" class="price price--non-sale"></span>
        </span> */

        let price_main_selector = Selector::parse("span.price--main").unwrap();
        let price_non_sale_selector = Selector::parse("span.price--non-sale").unwrap();

        let price_main =
            element_to_text(product_element.select(&price_main_selector).next().unwrap());

        let price_non_sale = element_to_text(
            product_element
                .select(&price_non_sale_selector)
                .next()
                .unwrap(),
        );

        let mut price = FirearmPrice {
            regular_price: 0,
            sale_price: None,
        };

        if price_non_sale == "" {
            price.regular_price = price_to_cents(price_main)?;
        } else {
            price.regular_price = price_to_cents(price_non_sale)?;
            price.sale_price = Some(price_to_cents(price_main)?);
        }

        Ok(price)
    }
}

#[async_trait]
impl Retailer for CanadasGunShop {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_param: &SearchParams,
    ) -> Result<Request, RetailerError> {
        let firearm_type = match search_param.firearm_type.unwrap() {
            FirearmType::Rifle => "rifles",
            FirearmType::Shotgun => "shotguns",
        };

        let action_type = match search_param.action_type.unwrap() {
            ActionType::SemiAuto => "Semi-Auto",
            ActionType::LeverAction => "Lever",
            ActionType::BreakAction => "Break",
            ActionType::BoltAction => "Bolt",
            ActionType::OverUnder => "",
            ActionType::PumpAction => "Pump",
            ActionType::SideBySide => "",
            ActionType::SingleShot => "",
            ActionType::Revolver => "Revolver",
            ActionType::StraightPull => "Straight-Pull",
        };

        let request: Request = RequestBuilder::new()
            .set_url(
                URL.replace("{firearm_type}", firearm_type)
                    .replace("{page_limit}", PAGE_LIMIT.to_string().as_str())
                    .replace("{page}", (page_num + 1).to_string().as_str())
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

        let product_selector = Selector::parse("li.product > article.card").unwrap();

        for product in html.select(&product_selector) {
            let name_link_selector = Selector::parse("h4.card-title > a").unwrap();
            let image_selector =
                Selector::parse("div.card-img-container > img.card-image").unwrap();

            let name_link_element: ElementRef<'_> =
                product.select(&name_link_selector).next().unwrap();

            let url = name_link_element.attr("href").unwrap();
            let name = element_to_text(name_link_element);
            let image = product
                .select(&image_selector)
                .next()
                .unwrap()
                .attr("src")
                .unwrap();

            let price = Self::get_price(product)?;

            let mut new_firearm =
                FirearmResult::new(name, url, price, RetailerName::CanadasGunShop);
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
                action_type: Some(ActionType::BoltAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::BreakAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::LeverAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::PumpAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::Revolver),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::SemiAuto),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::StraightPull),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Rifle),
            },
            // shotguns
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::BoltAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::BreakAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::LeverAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::PumpAction),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::Revolver),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::SemiAuto),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
            SearchParams {
                lookup: "",
                action_type: Some(ActionType::StraightPull),
                ammo_type: None,
                firearm_class: None,
                firearm_type: Some(FirearmType::Shotgun),
            },
        ]);

        Ok(params)
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(response);
        let count_selector = Selector::parse("div.pagination-info").unwrap();

        let Some(count_element) = html.select(&count_selector).next() else {
            return Ok(0);
        };

        let count_text = element_to_text(count_element);
        let regex = Regex::new(r"(\d+)\s+total$").unwrap();

        let item_count = regex
            .captures(&count_text)
            .unwrap()
            .get(1)
            .unwrap()
            .as_str();

        debug!("{:?}", item_count);

        Ok((item_count.parse::<u64>().unwrap() / PAGE_LIMIT).into())
    }

    fn get_crawler(&self) -> UnprotectedCrawler {
        self.crawler
    }

    fn get_page_cooldown(&self) -> u64 {
        PAGE_COOLDOWN
    }
}
