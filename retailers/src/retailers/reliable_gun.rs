use std::time::Duration;

use async_trait::async_trait;
use crawler::{
    request::Request,
    traits::{Crawler, HttpMethod},
    unprotected::UnprotectedCrawler,
};
use scraper::{ElementRef, Html, Selector};
use serde_json::Value;
use tokio::time::sleep;
use tracing::{debug, trace};

use crate::{
    results::{
        constants::{ActionType, AmmunitionType, FirearmClass, FirearmType, RetailerName},
        firearm::{FirearmPrice, FirearmResult},
    },
    traits::{Retailer, SearchParams},
    utils::price_to_cents,
};

/// looks like a single gun with swappable barrels
// 410 - https://www.reliablegun.com/combo-guns (non restricted centerfire?)
/// Rifle & Scope Combo
// 1052 https://www.reliablegun.com/rifle-scope-combo

const CRAWL_DELAY_SECS: u64 = 10; // https://www.reliablegun.com/robots.txt
const FILTER_STRING: &str = "{\"categoryId\":\"{catagory_id}\",\"manufacturerId\":\"0\",\"vendorId\":\"0\",\"pageNumber\":\"{page_number}\",\"orderby\":\"0\",\"viewmode\":\"grid\",\"pagesize\":\"12\",\"queryString\":\"\",\"shouldNotStartFromFirstPage\":true,\"keyword\":\"\",\"searchCategoryId\":\"0\",\"searchManufacturerId\":\"0\",\"searchVendorId\":\"0\",\"priceFrom\":\"\",\"priceTo\":\"\",\"includeSubcategories\":\"False\",\"searchInProductDescriptions\":\"False\",\"advancedSearch\":\"False\",\"isOnSearchPage\":\"False\"}";
const BASE_URL: &str = "https://www.reliablegun.com";
const HEADERS: [(&str, &str); 5] = [
    (
        "user-agent",
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36",
    ),
    ("accept", "*/*"),
    ("accept-language", "en-CA,en;q=0.9"),
    ("origin", "https://www.reliablegun.com"),
    ("referer", "https://www.reliablegun.com/firearms"),
];
const SEARCH_PARAMS: [SearchParams; 15] = [
    // centerfire
    SearchParams {
        lookup: "420", // https://www.reliablegun.com/semi-auto-rifles-2
        action_type: Some(ActionType::SemiAuto),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "412", // https://www.reliablegun.com/lever-action-rifles
        action_type: Some(ActionType::LeverAction),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "408", // https://www.reliablegun.com/break-action-rifles
        action_type: Some(ActionType::BreakAction),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "406", // https://www.reliablegun.com/bolt-action-rifles-2
        action_type: Some(ActionType::BoltAction),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "414", // https://www.reliablegun.com/over-under-shotguns
        action_type: Some(ActionType::OverUnder),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Shotgun),
    },
    SearchParams {
        lookup: "418", // https://www.reliablegun.com/pump-action-shotguns
        action_type: Some(ActionType::PumpAction),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Shotgun),
    },
    SearchParams {
        lookup: "422", // https://www.reliablegun.com/semi-auto-shotguns
        action_type: Some(ActionType::SemiAuto),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Shotgun),
    },
    SearchParams {
        lookup: "424", // https://www.reliablegun.com/side-by-side-shotguns
        action_type: Some(ActionType::SideBySide),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Shotgun),
    },
    SearchParams {
        lookup: "446", // https://www.reliablegun.com/bolt-shotguns
        action_type: Some(ActionType::BoltAction),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Shotgun),
    },
    SearchParams {
        lookup: "448", // https://www.reliablegun.com/single-shot-shotgun
        action_type: Some(ActionType::SingleShot),
        ammo_type: Some(AmmunitionType::CenterFire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Shotgun),
    },
    // rimfire
    SearchParams {
        lookup: "426", // https://www.reliablegun.com/bolt-action-rifles
        action_type: Some(ActionType::BoltAction),
        ammo_type: Some(AmmunitionType::Rimfire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "428", // https://www.reliablegun.com/lever-action-rifles-2
        action_type: Some(ActionType::LeverAction),
        ammo_type: Some(AmmunitionType::Rimfire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "425", // https://www.reliablegun.com/break-action-rifles-2
        action_type: Some(ActionType::BreakAction),
        ammo_type: Some(AmmunitionType::Rimfire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "432", // https://www.reliablegun.com/semi-auto-rifles
        action_type: Some(ActionType::SemiAuto),
        ammo_type: Some(AmmunitionType::Rimfire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "430", // https://www.reliablegun.com/pump-rifles-2
        action_type: Some(ActionType::PumpAction),
        ammo_type: Some(AmmunitionType::Rimfire),
        firearm_class: Some(FirearmClass::NonRestricted),
        firearm_type: Some(FirearmType::Rifle),
    },
];

pub struct ReliableGun {
    crawler: UnprotectedCrawler,
    headers: Vec<(String, String)>,
}

impl ReliableGun {
    pub fn new() -> ReliableGun {
        // do I need to recreate it here?
        // or can I just have the headers here and not in a const?
        ReliableGun {
            crawler: UnprotectedCrawler::new(),
            headers: HEADERS
                .to_vec()
                .into_iter()
                .map(|(key, value)| (key.to_string(), value.to_string()))
                .collect(),
        }
    }

    fn find_prices(element: ElementRef) -> FirearmPrice {
        let actual_selector = Selector::parse("span.actual-price").unwrap();
        let old_selector = Selector::parse("span.old-price").unwrap();

        let actual_price = price_to_cents(
            element
                .select(&actual_selector)
                .next()
                .unwrap()
                .text()
                .collect::<String>()
                .trim()
                .to_string(),
        );

        if let Some(old_price_element) = element.select(&old_selector).next() {
            let old_price = old_price_element
                .text()
                .collect::<String>()
                .trim()
                .to_string();

            FirearmPrice {
                regular_price: price_to_cents(old_price),
                sale_price: Some(actual_price),
            }
        } else {
            FirearmPrice {
                regular_price: actual_price,
                sale_price: None,
            }
        }
    }

    fn get_max_page_num(html: &str) -> Option<u32> {
        // <li class="last-page">
        // <a href="/getFilteredProducts?pagenumber=22">Last</a>
        // </li>

        let fragment = Html::parse_fragment(html);
        let last_page_selector = Selector::parse("li.last-page > a").unwrap();
        let next_page_selector = Selector::parse("li.next-page").unwrap();

        match fragment.select(&last_page_selector).next() {
            Some(element) => {
                debug!("Extracting last page button");

                let (_, page_num) = element
                    .attr("href")
                    .unwrap()
                    .trim()
                    .split_once("?pagenumber=")
                    .unwrap();

                let result = page_num.parse::<u32>().unwrap() + 1;

                debug!("Extracted final page number: {}", result);

                Some(result)
            }
            None => {
                debug!("Missing page element, checking for a page 2");

                match fragment.select(&next_page_selector).next() {
                    Some(_) => {
                        debug!("Only two pages, returning the last page");

                        Some(3)
                    }
                    None => {
                        debug!("Catagory only has a single page");

                        None
                    }
                }
            }
        }
    }

    fn get_firearms(html: &str, parameters: &SearchParams) -> Vec<FirearmResult> {
        let mut result: Vec<FirearmResult> = Vec::new();

        trace!("{}", html);

        let fragment = Html::parse_fragment(html);

        let description_selector = Selector::parse("div.description").unwrap();
        let url_selector = Selector::parse("h2.product-title > a").unwrap();
        let img_selector = Selector::parse("img.product-overview-img").unwrap();

        for element in fragment.select(&Selector::parse("div.product-item").unwrap()) {
            let description = element
                .select(&description_selector)
                .next()
                .unwrap()
                .text()
                .collect::<String>()
                .trim()
                .to_string();

            let url_element = element.select(&url_selector).next().unwrap();
            let url_href = url_element.attr("href").unwrap();
            let name = url_element.text().collect::<String>().trim().to_string();

            let image_url = element
                .select(&img_selector)
                .next()
                .unwrap()
                .attr("src")
                .unwrap();

            let price = Self::find_prices(element);

            let mut firearm = FirearmResult::new(
                name,
                format!("{}{}", BASE_URL, url_href),
                price,
                RetailerName::ReliableGun,
            );
            firearm.thumbnail_link = Some(image_url.to_string());
            firearm.description = Some(description);
            firearm.action_type = parameters.action_type;
            firearm.ammo_type = parameters.ammo_type;
            firearm.firearm_class = parameters.firearm_class;
            firearm.firearm_type = parameters.firearm_type;

            result.push(firearm);
        }

        result
    }

    async fn send_request(&self, page_num: &str, parameters: &SearchParams<'_>) -> String {
        let filter = FILTER_STRING
            .replace("{catagory_id}", parameters.lookup)
            .replace("{page_number}", page_num);

        debug!("Setting filter string to {}", filter);

        let json = serde_json::from_str::<Value>(filter.as_str()).unwrap();

        let request_builder = Request::builder()
            .set_method(HttpMethod::POST)
            .set_url("https://www.reliablegun.com/getFilteredProducts")
            .set_json_body(json)
            .set_headers(&self.headers);

        debug!("Sending request to page {}", page_num);

        self.crawler
            .make_web_request(request_builder.build())
            .await
            .unwrap()
    }
}

#[async_trait]
impl Retailer for ReliableGun {
    async fn get_firearms(&self) -> Vec<FirearmResult> {
        let mut firearms: Vec<FirearmResult> = Vec::new();

        for parameters in SEARCH_PARAMS {
            let response = self.send_request("1", &parameters).await;
            let html = response.as_str();

            firearms.append(&mut Self::get_firearms(html, &parameters));

            if let Some(page_num) = Self::get_max_page_num(html) {
                for i in 2..page_num {
                    sleep(Duration::from_secs(CRAWL_DELAY_SECS)).await;

                    let response = self.send_request(i.to_string().as_str(), &parameters).await;
                    let html = response.as_str();

                    firearms.append(&mut Self::get_firearms(html, &parameters));
                }
            }

            sleep(Duration::from_secs(1)).await;
        }

        firearms
    }
}
