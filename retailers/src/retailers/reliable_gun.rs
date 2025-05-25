use async_trait::async_trait;
use crawler::{request::Request, traits::HttpMethod, unprotected::UnprotectedCrawler};
use scraper::{ElementRef, Html, Selector};
use serde_json::Value;
use tracing::{debug, error};

use crate::{
    errors::RetailerError,
    results::{
        constants::{ActionType, AmmunitionType, FirearmClass, FirearmType, RetailerName},
        firearm::{FirearmPrice, FirearmResult},
    },
    traits::{Retailer, SearchParams},
    utils::{
        conversions::{price_to_cents, string_to_u64},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

/// looks like a single gun with swappable barrels
// 410 - https://www.reliablegun.com/combo-guns (non restricted centerfire?)
/// Rifle & Scope Combo
// 1052 https://www.reliablegun.com/rifle-scope-combo

const CRAWL_DELAY_SECS: u64 = 10; // https://www.reliablegun.com/robots.txt
const PAGE_SIZE: u64 = 12; // Reliable Gun's site is slow
const FILTER_STRING: &str = "{\"categoryId\":\"{catagory_id}\",\"manufacturerId\":\"0\",\"vendorId\":\"0\",\"pageNumber\":\"{page_number}\",\"orderby\":\"0\",\"viewmode\":\"grid\",\"pagesize\":\"{page_size}\",\"queryString\":\"\",\"shouldNotStartFromFirstPage\":true,\"keyword\":\"\",\"searchCategoryId\":\"0\",\"searchManufacturerId\":\"0\",\"searchVendorId\":\"0\",\"priceFrom\":\"\",\"priceTo\":\"\",\"includeSubcategories\":\"False\",\"searchInProductDescriptions\":\"False\",\"advancedSearch\":\"False\",\"isOnSearchPage\":\"False\"}";
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

    fn find_prices(element: ElementRef) -> Result<FirearmPrice, RetailerError> {
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
        )?;

        if let Some(old_price_element) = element.select(&old_selector).next() {
            let old_price = old_price_element
                .text()
                .collect::<String>()
                .trim()
                .to_string();

            Ok(FirearmPrice {
                regular_price: price_to_cents(old_price)?,
                sale_price: Some(actual_price),
            })
        } else {
            Ok(FirearmPrice {
                regular_price: actual_price,
                sale_price: None,
            })
        }
    }

    fn extract_page_num_from_href(element: ElementRef) -> Result<u64, RetailerError> {
        let href = element_extract_attr(element, "href".into())?;

        let Some((_, page_num)) = href.split_once("?pagenumber=") else {
            let message = format!("href element is missing pagenumber query: {}", href);

            error!(message);

            return Err(RetailerError::GeneralError(message));
        };

        let max_pages = string_to_u64(page_num.to_string())?;

        Ok(max_pages)
    }
}

#[async_trait]
impl Retailer for ReliableGun {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_param: &SearchParams,
    ) -> Result<Request, RetailerError> {
        let filter = FILTER_STRING
            .replace("{catagory_id}", search_param.lookup)
            .replace("{page_number}", (page_num + 1).to_string().as_str())
            .replace("{page_size}", PAGE_SIZE.to_string().as_str());

        debug!("Setting filter string to {}", filter);

        let Ok(json) = serde_json::from_str::<Value>(filter.as_str()) else {
            error!("Failed to convert string into Value: {}", filter);

            return Err(RetailerError::InvalidRequestBody(filter));
        };

        let request_builder = Request::builder()
            .set_method(HttpMethod::POST)
            .set_url("https://www.reliablegun.com/getFilteredProducts")
            .set_json_body(json)
            .set_headers(&self.headers);

        debug!("Sending request to page {}", page_num);

        Ok(request_builder.build())
    }

    async fn parse_response(
        &self,
        response: &String,
        search_param: &SearchParams,
    ) -> Result<Vec<FirearmResult>, RetailerError> {
        let mut result: Vec<FirearmResult> = Vec::new();

        let fragment = Html::parse_fragment(response);

        for element in fragment.select(&Selector::parse("div.product-item").unwrap()) {
            let description_element =
                extract_element_from_element(element, "div.description".into())?;
            let url_element = extract_element_from_element(element, "h2.product-title > a".into())?;
            let image_element =
                extract_element_from_element(element, "img.product-overview-img".into())?;

            let description = element_to_text(description_element);
            let url_href = element_extract_attr(url_element, "href".into())?;
            let name = element_to_text(url_element);
            let image_url = element_extract_attr(image_element, "src".into())?;

            let price = Self::find_prices(element)?;

            let mut firearm = FirearmResult::new(
                name,
                format!("{}{}", BASE_URL, url_href),
                price,
                RetailerName::ReliableGun,
            );
            firearm.thumbnail_link = Some(image_url.to_string());
            firearm.description = Some(description);
            firearm.action_type = search_param.action_type;
            firearm.ammo_type = search_param.ammo_type;
            firearm.firearm_class = search_param.firearm_class;
            firearm.firearm_type = search_param.firearm_type;

            result.push(firearm);
        }

        Ok(result)
    }

    fn get_search_parameters(&self) -> Result<Vec<SearchParams>, RetailerError> {
        let params = Vec::from_iter([
            // centerfire
            SearchParams {
                lookup: "412", // https://www.reliablegun.com/lever-action-rifles
                action_type: Some(ActionType::LeverAction),
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
                lookup: "420", // https://www.reliablegun.com/semi-auto-rifles-2
                action_type: Some(ActionType::SemiAuto),
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
        ]);

        Ok(params)
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        // <li class="last-page"><a href="/getFilteredProducts?pagenumber=22">Last</a></li>

        let html = Html::parse_fragment(response);
        let last_page_selector = Selector::parse("li.last-page > a").unwrap();
        let individual_page_selector = Selector::parse("li.individual-page > a").unwrap();

        // the reliable gun pagination elements behave weird
        match html.select(&last_page_selector).next() {
            Some(final_page_element) => {
                debug!("Extracting last page button");

                let max_pages = Self::extract_page_num_from_href(final_page_element)?;

                debug!("Extracted final page number: {}", max_pages);

                Ok(max_pages)
            }
            None => {
                debug!("Missing last page element, checking for complete page links");
                // <ul>
                //<li class="current-page"><span>1</span></li>
                //<li class="individual-page"><a href="/getFilteredProducts?pagenumber=2">2</a></li>
                //<li class="individual-page"><a href="/getFilteredProducts?pagenumber=3">3</a></li>
                //<li class="next-page"><a href="/getFilteredProducts?pagenumber=2">Next</a></li>
                //</ul>
                match html.select(&individual_page_selector).last() {
                    Some(max_page) => {
                        let max_pages_count = Self::extract_page_num_from_href(max_page)?;

                        debug!("Extracted non last page element: {}", max_pages_count);

                        Ok(max_pages_count)
                    }
                    None => {
                        debug!("Catagory only has a single page");

                        Ok(0)
                    }
                }
            }
        }
    }

    fn get_crawler(&self) -> UnprotectedCrawler {
        self.crawler
    }

    fn get_page_cooldown(&self) -> u64 {
        CRAWL_DELAY_SECS
    }
}
