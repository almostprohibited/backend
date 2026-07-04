use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::{
    WebClient,
    request::{Request, RequestBuilder},
};
use scraper::{ElementRef, Html, Selector};
use tracing::{debug, info};

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        conversions::{price_to_cents, string_to_u64},
        generic_sitemap::get_search_queries,
        helpers::clean_url,
        html::{element_extract_attr, element_to_text, extract_element_from_element},
        sucuri_cookie::get_sucuri_cookie,
    },
};

const SITE_MAP: &str = "https://latulippe.com/Content/SiteMap/sitemap.xml";
const BASE_URL: &str = "https://latulippe.com/en/";
const URL: &str = "https://latulippe.com/en/{category}/?page={page}";

pub struct Latulippe {
    search_queries: Vec<HtmlSearchQuery>,
}

impl Default for Latulippe {
    fn default() -> Self {
        Self::new()
    }
}

impl Latulippe {
    pub fn new() -> Self {
        Self {
            search_queries: vec![],
        }
    }
}

impl HtmlRetailerSuper for Latulippe {}

#[async_trait]
impl Retailer for Latulippe {
    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::Latulippe
    }

    async fn init(&mut self) -> Result<(), RetailerError> {
        let cookie = get_sucuri_cookie(BASE_URL).await?;

        WebClient::set_cookie(BASE_URL, &cookie);

        debug!("Using cookie: {cookie}");

        // not using constant for domain here since
        // their antibot logic works kinda weird and
        // treats subdomains different
        let queries = get_search_queries(SITE_MAP, "https://www.latulippe.com/en/", |link| {
            if !link.starts_with("catalog") {
                return None;
            }

            if [
                "catalog/shooting/hunting-rifles",
                "catalog/shooting/handguns",
                "catalog/shooting/shotguns",
                "catalog/shooting/black-powder-guns",
            ]
            .contains(&link.as_str())
            {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Firearm,
                });
            } else if link == "catalog/shooting/ammunition" {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Ammunition,
                });
            } else if [
                "catalog/shooting/shooting-accessories",
                "catalog/shooting/scopes-and-accessories",
                "catalog/shooting/reloading",
                "catalog/shooting/gun-cleaning",
                "catalog/shooting/gun-and-ammo-storage",
                "catalog/shooting/firearm-parts",
                "catalog/shooting/black-powder",
            ]
            .contains(&link.as_str())
            {
                return Some(HtmlSearchQuery {
                    term: link,
                    category: Category::Other,
                });
            }

            None
        })
        .await?;

        self.search_queries.extend(queries);

        Ok(())
    }
}

#[async_trait]
impl HtmlRetailer for Latulippe {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", (page_num + 1).to_string().as_str());

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

        let fragment = Html::parse_document(response);

        let product_selector = Selector::parse("ul.produits > li").unwrap();

        for element in fragment.select(&product_selector) {
            if element_extract_attr(element, "data-gaec-id").is_err() {
                info!("Skipping element that is not product");
                continue;
            }

            let title_element = extract_element_from_element(element, "div.titre")?;
            let image_element = extract_element_from_element(element, "div.image img")?;

            let name = format!(
                "{} {}",
                element_extract_attr(element, "data-gaec-brand")?,
                element_extract_attr(element, "data-gaec-name")?
            );

            let url = format!(
                "https://latulippe.com{}",
                element_extract_attr(element, "data-url")?
            );

            let image_url = clean_url(&get_image(image_element)?);

            let price_element =
                extract_element_from_element(title_element, "div[class^='ui-tag-discount']")?;

            let sale_element = extract_element_from_element(price_element, "b")?;
            let sale_price = price_to_cents(element_to_text(sale_element))?;

            let regular_element = extract_element_from_element(price_element, "small")?;
            let regular_price = price_to_cents(element_to_text(regular_element))?;

            let mut price = Price {
                regular_price,
                sale_price: None,
            };

            if sale_price != regular_price {
                price.sale_price = Some(sale_price);
            }

            let result = CrawlResult::new(
                name,
                url,
                price,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(image_url);

            results.push(result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        self.search_queries.clone()
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let html = Html::parse_document(response);

        let page_number_selector = Selector::parse("div.pagination > ul > li:not(.next)").unwrap();

        let mut page_links = html.select(&page_number_selector);

        let Some(last_page_element) = page_links.next_back() else {
            return Ok(0);
        };

        string_to_u64(element_to_text(last_page_element))
    }
}

// logic copied from lightspeed parser
fn get_image(image_element: ElementRef) -> Result<String, RetailerError> {
    if let Ok(data_src) = element_extract_attr(image_element, "data-src")
        && data_src.starts_with("https")
        && !data_src.contains("lazy")
    {
        return Ok(data_src);
    };

    if let Ok(regular_src) = element_extract_attr(image_element, "src")
        && regular_src.starts_with("https")
        && !regular_src.contains("lazy")
    {
        return Ok(regular_src);
    }

    Err(RetailerError::HtmlElementMissingAttribute(
        "'valid data-src or src'".into(),
        element_to_text(image_element),
    ))
}
