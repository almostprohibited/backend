use async_trait::async_trait;
use crawler::{
    request::{Request, RequestBuilder},
    unprotected::UnprotectedCrawler,
};
use scraper::{ElementRef, Html, Selector};
use tracing::{debug, error};

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

const URL: &str = "https://leverarms.com/product-category/guns/{catagory}/page/{page}/";

pub struct LeverArms {
    crawler: UnprotectedCrawler,
    retailer: RetailerName,
}

impl LeverArms {
    pub fn new() -> Self {
        Self {
            crawler: UnprotectedCrawler::new(),
            retailer: RetailerName::LeverArms,
        }
    }

    fn parse_firearm(
        &self,
        element: ElementRef,
        search_param: &SearchParams<'_>,
    ) -> Result<FirearmResult, RetailerError> {
        let title_element =
            extract_element_from_element(element, "h2.woocommerce-loop-product__title".into())?;
        let price_element =
            extract_element_from_element(element, "span.woocommerce-Price-amount".into())?;
        let image_element =
            extract_element_from_element(element, "img.attachment-woocommerce_thumbnail".into())?;

        let link = element_extract_attr(element, "href".into())?;
        let title = element_to_text(title_element);
        let price = price_to_cents(element_to_text(price_element))?;
        let image_link = element_extract_attr(image_element, "src".into())?;

        let mut result = FirearmResult::new(
            title,
            link,
            FirearmPrice {
                regular_price: price,
                sale_price: None,
            },
            self.get_retailer_name(),
        );
        result.thumbnail_link = Some(image_link.to_string());
        result.action_type = search_param.action_type;
        result.ammo_type = search_param.ammo_type;
        result.firearm_class = search_param.firearm_class;
        result.firearm_type = search_param.firearm_type;

        Ok(result)
    }
}

#[async_trait]
impl Retailer for LeverArms {
    fn get_retailer_name(&self) -> RetailerName {
        self.retailer
    }

    async fn build_page_request(
        &self,
        page_num: u64,
        search_param: &SearchParams,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{catagory}", search_param.lookup)
            .replace("{page}", (page_num + 1).to_string().as_str());

        debug!("Setting page to {}", url);

        let request = RequestBuilder::new().set_url(url).build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_param: &SearchParams,
    ) -> Result<Vec<FirearmResult>, RetailerError> {
        let mut firearms: Vec<FirearmResult> = Vec::new();

        let fragment = Html::parse_document(&response);

        let product_selector = Selector::parse("a.woocommerce-LoopProduct-link").unwrap();

        for element in fragment.select(&product_selector) {
            firearms.push(self.parse_firearm(element, search_param)?);
        }

        Ok(firearms)
    }

    fn get_search_parameters(&self) -> Result<Vec<SearchParams>, RetailerError> {
        let params = Vec::from_iter([
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
        ]);

        Ok(params)
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let fragment = Html::parse_document(&response);
        let page_number_selector = Selector::parse("a.page-numbers").unwrap();

        let mut page_links = fragment.select(&page_number_selector);
        let page_links_count = page_links.clone().count();

        if page_links_count == 0 {
            // indicates no pages
            return Ok(0);
        }

        // page links look like:
        // ["1", "2", "3", "->"]
        // do `count - 2` to grab the number before the arrow
        let Some(last_page_element) = page_links.nth(page_links_count - 2) else {
            let message = format!("Invalid number of page elements: {:?}", page_links);
            error!(message);

            return Err(RetailerError::GeneralError(
                "Invalid number of page elements".into(),
            ));
        };

        Ok(string_to_u64(element_to_text(last_page_element))?)
    }

    fn get_crawler(&self) -> UnprotectedCrawler {
        self.crawler
    }

    fn get_page_cooldown(&self) -> u64 {
        1
    }
}
