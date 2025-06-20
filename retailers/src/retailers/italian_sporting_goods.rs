use async_trait::async_trait;
use crawler::{request::RequestBuilder, traits::Crawler, unprotected::UnprotectedCrawler};
use scraper::{ElementRef, Html, Selector};
use tokio::time::{Duration, sleep};
use tracing::{debug, trace};

use crate::traits::Retailer;

const URL: &str =
    "https://www.italiansportinggoods.com/firearms/{catagory}.html?product_list_limit=25?p={page}";
const SEARCH_PARAMS: [SearchParams; 16] = [
    // centerfire rifle
    SearchParams {
        lookup: "centerfire-rifles/bolt-action", // https://www.italiansportinggoods.com/firearms/centerfire-rifles/bolt-action.html
        action_type: Some(ActionType::BoltAction),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "centerfire-rifles/lever-action", // https://www.italiansportinggoods.com/firearms/centerfire-rifles/lever-action.html
        action_type: Some(ActionType::LeverAction),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "centerfire-rifles/pump-action", // https://www.italiansportinggoods.com/firearms/centerfire-rifles/pump-action.html
        action_type: Some(ActionType::PumpAction),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "centerfire-rifles/semi-automatic", // https://www.italiansportinggoods.com/firearms/centerfire-rifles/semi-automatic.html
        action_type: Some(ActionType::SemiAuto),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "centerfire-rifles/break", // https://www.italiansportinggoods.com/firearms/centerfire-rifles/break.html
        action_type: Some(ActionType::BreakAction),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "centerfire-rifles/over-under", // https://www.italiansportinggoods.com/firearms/centerfire-rifles/over-under.html
        action_type: Some(ActionType::OverUnder),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    // rimfire
    SearchParams {
        lookup: "rimfire-rifles/bolt-action", // https://www.italiansportinggoods.com/firearms/rimfire-rifles/bolt-action.html
        action_type: Some(ActionType::BoltAction),
        ammo_type: Some(AmmunitionType::Rimfire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "rimfire-rifles/lever-action", // https://www.italiansportinggoods.com/firearms/rimfire-rifles/lever-action.html
        action_type: Some(ActionType::LeverAction),
        ammo_type: Some(AmmunitionType::Rimfire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "rimfire-rifles/semi-auto", // https://www.italiansportinggoods.com/firearms/rimfire-rifles/semi-auto.html
        action_type: Some(ActionType::SemiAuto),
        ammo_type: Some(AmmunitionType::Rimfire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "rimfire-rifles/revolver", // https://www.italiansportinggoods.com/firearms/rimfire-rifles/revolver.html
        action_type: Some(ActionType::SemiAuto),
        ammo_type: Some(AmmunitionType::Rimfire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    // shotguns
    SearchParams {
        lookup: "shotguns/lever", // https://www.italiansportinggoods.com/firearms/shotguns/lever.html
        action_type: Some(ActionType::LeverAction),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Shotgun),
    },
    SearchParams {
        lookup: "shotguns/over-and-under", // https://www.italiansportinggoods.com/firearms/shotguns/over-and-under.html
        action_type: Some(ActionType::OverUnder),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Shotgun),
    },
    SearchParams {
        lookup: "shotguns/side-by-side", // https://www.italiansportinggoods.com/firearms/shotguns/side-by-side.html
        action_type: Some(ActionType::SideBySide),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Shotgun),
    },
    SearchParams {
        lookup: "shotguns/pump-action", // https://www.italiansportinggoods.com/firearms/shotguns/pump-action.html
        action_type: Some(ActionType::PumpAction),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Shotgun),
    },
    SearchParams {
        lookup: "shotguns/semi-automatic", // https://www.italiansportinggoods.com/firearms/shotguns/semi-automatic.html
        action_type: Some(ActionType::SemiAuto),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Shotgun),
    },
    SearchParams {
        lookup: "shotguns/single-shot", // https://www.italiansportinggoods.com/firearms/shotguns/single-shot.html
        action_type: Some(ActionType::SingleShot),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Shotgun),
    },
];

pub struct ItalianSportingGoods {
    crawler: UnprotectedCrawler,
}

impl ItalianSportingGoods {
    pub fn new() -> ItalianSportingGoods {
        ItalianSportingGoods {
            crawler: UnprotectedCrawler::new(),
        }
    }

    fn get_num_pages(html: &String) -> u32 {
        let fragment = Html::parse_document(&html);

        let item_counts = Selector::parse("p#toolbar-amount > span.toolbar-number").unwrap();
        if let Some(total_items_element) = fragment.select(&item_counts).nth(2) {
            let count = total_items_element
                .text()
                .collect::<String>()
                .trim()
                .parse::<f32>()
                .unwrap();

            (count / 25.).ceil() as u32 + 1
        } else {
            0
        }
    }

    fn parse_prices(element: ElementRef) -> FirearmPrice {
        let final_price =
            Selector::parse("span.price-wrapper[data-price-type=finalPrice]").unwrap();
        let old_price = Selector::parse("span.price-wrapper[data-price-type=oldPrice]").unwrap();

        let final_price = price_to_cents(
            element
                .select(&final_price)
                .next()
                .unwrap()
                .attr("data-price-amount")
                .unwrap()
                .to_string(),
        );

        if let Some(old_price_element) = element.select(&old_price).next() {
            let old_price = old_price_element
                .attr("data-price-amount")
                .unwrap()
                .to_string();

            FirearmPrice {
                regular_price: price_to_cents(old_price),
                sale_price: Some(final_price),
            }
        } else {
            FirearmPrice {
                regular_price: final_price,
                sale_price: None,
            }
        }
    }

    fn parse_webpage(html: &String, parameters: &SearchParams) -> Vec<FirearmResult> {
        let mut firearms: Vec<FirearmResult> = Vec::new();

        let fragment = Html::parse_document(&html);

        trace!(html);

        let product_selector = Selector::parse("div.product-item-details").unwrap();
        let name_link_selector = Selector::parse("a.product-item-link").unwrap();

        for element in fragment.select(&product_selector) {
            let ahref = element.select(&name_link_selector).next().unwrap();

            if let Some(link) = ahref.attr("href") {
                let name = ahref.text().collect::<String>().trim().to_string();

                let prices = Self::parse_prices(element);

                let mut new_firearm =
                    FirearmResult::new(name, link, prices, RetailerName::ItalianSportingGoods);
                new_firearm.action_type = parameters.action_type;
                new_firearm.ammo_type = parameters.ammo_type;
                new_firearm.firearm_class = parameters.firearm_class;
                new_firearm.firearm_type = parameters.firearm_type;

                firearms.push(new_firearm);
            }
        }

        firearms
    }

    async fn send_request(&self, page_num: &str, parameters: &SearchParams<'_>) -> String {
        let url = URL
            .replace("{catagory}", parameters.lookup)
            .replace("{page}", page_num);

        debug!("Setting page to {}", url);

        let request = RequestBuilder::new().set_url(url).build();

        self.crawler.make_web_request(request).await.unwrap()
    }
}

#[async_trait]
impl Retailer for ItalianSportingGoods {
    async fn get_firearms(&self) -> Vec<FirearmResult> {
        let mut firearms: Vec<FirearmResult> = Vec::new();

        for search in SEARCH_PARAMS {
            let result = self.send_request("1", &search).await;

            let mut new_firearms = Self::parse_webpage(&result, &search);

            for page_num in 2..Self::get_num_pages(&result) {
                sleep(Duration::from_secs(1)).await;

                let result = self
                    .send_request(page_num.to_string().as_str(), &search)
                    .await;

                new_firearms.append(&mut Self::parse_webpage(&result, &search));
            }

            firearms.append(&mut new_firearms);

            sleep(Duration::from_secs(1)).await;
        }

        firearms
    }
}
