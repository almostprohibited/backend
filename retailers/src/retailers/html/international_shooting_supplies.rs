use async_trait::async_trait;
use common::result::{
    base::CrawlResult,
    enums::{Category, RetailerName},
};
use crawler::request::{Request, RequestBuilder};
use scraper::{Html, Selector};
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::ecommerce::woocommerce::{WooCommerce, WooCommerceBuilder},
};

const URL: &str = "https://internationalshootingsupplies.com/product-category/{category}/page/{page}/?filter_stock_status=instock";

pub struct InternationalShootingSupplies;

impl InternationalShootingSupplies {
    pub fn new() -> Self {
        Self {}
    }
}

impl HtmlRetailerSuper for InternationalShootingSupplies {}

impl Retailer for InternationalShootingSupplies {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::InternationalShootingSupplies
    }
}

#[async_trait]
impl HtmlRetailer for InternationalShootingSupplies {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", &(page_num + 1).to_string());

        debug!("Setting page to {}", url);

        let request = RequestBuilder::new().set_url(url).build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let html = Html::parse_document(response);

        let product_selector = Selector::parse("ul.products > li.product.instock").unwrap();

        let woocommerce_helper = WooCommerceBuilder::default()
            .with_product_url_selector("div.astra-shop-summary-wrap > a.ast-loop-product__link")
            .with_product_name_selector(
                "div.astra-shop-summary-wrap > a.ast-loop-product__link > h2.woocommerce-loop-product__title"
            )
            .with_image_url_selector("a.woocommerce-LoopProduct-link > img")
            .build();

        for product in html.select(&product_selector) {
            let new_product = woocommerce_helper.parse_product(
                product,
                self.get_retailer_name(),
                search_term.category,
            )?;

            results.push(new_product);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        let mut search_params: Vec<HtmlSearchQuery> = Vec::new();

        [
            // "firearms/handguns",
            "firearms/rifles",
            "firearms/shotguns",
        ]
        .iter()
        .for_each(|term| {
            search_params.push(HtmlSearchQuery {
                term: term.to_string(),
                category: Category::Firearm,
            });
        });

        [
            "ammunition/handgun-ammo",
            "ammunition/rifle-ammo",
            "ammunition/rimfire-ammo",
            "ammunition/shotgun-ammo",
            "shooting-accessories/shooting-accessories-black-powder",
            "shooting-accessories/snap-caps", // where do I put snap caps?? its technically not "ammo"
        ]
        .iter()
        .for_each(|term| {
            search_params.push(HtmlSearchQuery {
                term: term.to_string(),
                category: Category::Ammunition,
            });
        });

        [
            "optics/mfr-binocular-accessories",
            "optics/night-vision",
            "optics/optics-rifelscope-accessories",
            "optics/optics-spotting-scope-accessories",
            "optics/binoculars",
            "optics/mounts",
            "optics/pistol-scopes",
            "optics/range-finders",
            "optics/red-dots-reflex",
            "optics/rifle-scopes",
            "optics/rimfire-scopes",
            "optics/sights",
            "optics/spotting-scopes-and-accessories",
            "parts/actions-and-action-parts",
            "parts/barrels",
            "parts/internal-stability-upgrades",
            "parts/magazine-upgrade-parts",
            "parts/stock-and-chassis-systems",
            "parts/trigger-assemblies-and-parts",
            "parts/buttstocks",
            "reloading-components/brass",
            "reloading-components/powder",
            "reloading-components/primers",
            "reloading-components/projectiles",
            "reloading-equipment/reloading-equip-projectile-production",
            "reloading-equipment/reloading-equip-reloading-component-dispensers",
            "reloading-equipment/reloading-accessories-misc",
            "reloading-equipment/case-cleaning-prep",
            "reloading-equipment/dies-die-accessories",
            "reloading-equipment/hand-tools",
            "reloading-equipment/kits",
            "reloading-equipment/presses-accessories",
            "reloading-equipment/reloading-equip-scales-measures-gauges",
            "reloading-equipment/trimmers",
            "reloading-equipment/tumblers",
            "shooting-accessories/shooting-accessories-apparel",
            // "shooting-accessories/achery-accessories",
            "shooting-accessories/buttstock-accessories",
            "shooting-accessories/shooting-accessories-cleaning-supplies-consumables",
            "shooting-accessories/gun-cabinets-safes",
            "shooting-accessories/gun-cases",
            // "shooting-accessories/shooting-accessories-hats",
            "shooting-accessories/shooting-accessories-muzzle-devices-externally-attached",
            "shooting-accessories/misc-shooting-accessories",
            "shooting-accessories/ammo-storage-locking-devices",
            "shooting-accessories/bags-pouches",
            "shooting-accessories/bipods-adapters",
            "shooting-accessories/shooting-accessories-chronographs-recording-tools",
            "shooting-accessories/cleaning-supplies-reusable",
            "shooting-accessories/eye-protection",
            "shooting-accessories/flashlights-lasers",
            "shooting-accessories/forends",
            "shooting-accessories/grips",
            "shooting-accessories/gun-parts",
            "shooting-accessories/gunsmithing",
            "shooting-accessories/hearing-protection",
            "shooting-accessories/holsters",
            "shooting-accessories/magazines-accessories",
            "shooting-accessories/shooting-rests",
            "shooting-accessories/slings-sling-hardware",
            "shooting-accessories/targets-accessories",
        ]
        .iter()
        .for_each(|term| {
            search_params.push(HtmlSearchQuery {
                term: term.to_string(),
                category: Category::Other,
            });
        });

        search_params
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        WooCommerce::parse_max_pages(response)
    }
}
