use async_trait::async_trait;
use crawler::{request::RequestBuilder, traits::Crawler, unprotected::UnprotectedCrawler};
use scraper::{ElementRef, Html, Selector};
use tokio::time::{Duration, sleep};
use tracing::debug;

use crate::{
    results::{
        constants::{ActionType, FirearmType, RetailerName},
        firearm::{FirearmPrice, FirearmResult},
    },
    traits::{Retailer, SearchParams},
    utils::{element_to_text, price_to_cents},
};

const URL: &str = "https://leverarms.com/product-category/guns/{catagory}/page/{page}/";
const SEARCH_PARAMS: [SearchParams; 5] = [
    // rifles
    SearchParams {
        lookup: "rifles/semi-auto-rifles", // https://leverarms.com/product-category/guns/rifles/semi-auto-rifles/
        action_type: Some(ActionType::SemiAuto),
        ammo_type: None,
        firearm_class: None,
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "bolt-action-rifles", // https://leverarms.com/product-category/guns/rifles/bolt-action-rifles/
        action_type: Some(ActionType::BoltAction),
        ammo_type: None,
        firearm_class: None,
        firearm_type: Some(FirearmType::Rifle),
    },
    SearchParams {
        lookup: "rifles/lever-action-rifles", // https://leverarms.com/product-category/guns/rifles/lever-action-rifles/
        action_type: Some(ActionType::LeverAction),
        ammo_type: None,
        firearm_class: None,
        firearm_type: Some(FirearmType::Rifle),
    },
    // shotguns
    SearchParams {
        lookup: "shotguns/pump-action-shotguns", // https://leverarms.com/product-category/guns/shotguns/pump-action-shotguns/
        action_type: Some(ActionType::PumpAction),
        ammo_type: None,
        firearm_class: None,
        firearm_type: Some(FirearmType::Shotgun),
    },
    SearchParams {
        lookup: "shotguns/over-under-shotguns", // https://leverarms.com/product-category/guns/shotguns/over-under-shotguns/
        action_type: Some(ActionType::OverUnder),
        ammo_type: None,
        firearm_class: None,
        firearm_type: Some(FirearmType::Shotgun),
    },
];

pub struct LeverArms {
    crawler: UnprotectedCrawler,
}

impl LeverArms {
    pub fn new() -> Self {
        Self {
            crawler: UnprotectedCrawler::new(),
        }
    }

    async fn send_request(&self, page_number: u64, parameters: &SearchParams<'_>) -> String {
        let url = URL
            .replace("{catagory}", parameters.lookup)
            .replace("{page}", page_number.to_string().as_str());

        debug!("Setting page to {}", url);

        let request = RequestBuilder::new().set_url(url).build();

        self.crawler.make_web_request(request).await.unwrap()
    }

    fn find_max_pages(html: String) -> u64 {
        let fragment = Html::parse_document(&html);
        let page_number_selector = Selector::parse("a.page-numbers").unwrap();

        let mut page_links = fragment.select(&page_number_selector);
        let page_links_count = page_links.clone().count();

        if page_links_count == 0 {
            // TODO: return Option instead of 2, this is suppose to indicate "no pages"
            2
        } else {
            // page links look like:
            // 1 2 3 ->
            // do `count - 2` to grab the number before the arrow
            page_links
                .nth(page_links_count - 2)
                .unwrap()
                .text()
                .collect::<String>()
                .trim()
                .parse::<u64>()
                .unwrap()
                + 1
        }
    }

    fn parse_firearm(element: ElementRef, search_param: &SearchParams<'_>) -> FirearmResult {
        let title_selector = Selector::parse("h2.woocommerce-loop-product__title").unwrap();
        let price_selector = Selector::parse("span.woocommerce-Price-amount").unwrap();
        let image_selector = Selector::parse("img.attachment-woocommerce_thumbnail").unwrap();

        let link = element.attr("href").unwrap();
        let title = element_to_text(element.select(&title_selector).next().unwrap());
        let price = price_to_cents(element_to_text(
            element.select(&price_selector).next().unwrap(),
        ));
        let image_link = element
            .select(&image_selector)
            .next()
            .unwrap()
            .attr("src")
            .unwrap();

        let mut result = FirearmResult::new(
            title,
            link,
            FirearmPrice {
                regular_price: price,
                sale_price: None,
            },
            RetailerName::LeverArms,
        );
        result.thumbnail_link = Some(image_link.to_string());
        result.action_type = search_param.action_type;
        result.ammo_type = search_param.ammo_type;
        result.firearm_class = search_param.firearm_class;
        result.firearm_type = search_param.firearm_type;

        result
    }

    fn parse_webpage(html: &String, search_param: &SearchParams<'_>) -> Vec<FirearmResult> {
        let mut firearms: Vec<FirearmResult> = Vec::new();

        let fragment = Html::parse_document(&html);

        let product_selector = Selector::parse("a.woocommerce-LoopProduct-link").unwrap();

        for element in fragment.select(&product_selector) {
            firearms.push(Self::parse_firearm(element, search_param));
        }

        firearms
    }
}

#[async_trait]
impl Retailer for LeverArms {
    async fn get_firearms(&self) -> Vec<FirearmResult> {
        let mut firearms: Vec<FirearmResult> = Vec::new();

        for search in SEARCH_PARAMS {
            let result = self.send_request(1, &search).await;

            let mut new_firearms = Self::parse_webpage(&result, &search);

            for page_num in 2..Self::find_max_pages(result) {
                sleep(Duration::from_secs(1)).await;

                let result = self.send_request(page_num, &search).await;

                new_firearms.append(&mut Self::parse_webpage(&result, &search));
            }

            firearms.append(&mut new_firearms);

            sleep(Duration::from_secs(1)).await;
        }

        firearms
    }
}
