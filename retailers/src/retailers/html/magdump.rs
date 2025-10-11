use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::{
    request::{Request, RequestBuilder},
    unprotected::UnprotectedCrawler,
};
use scraper::{ElementRef, Html, Selector};
use serde::Deserialize;
use tracing::debug;

use crate::{
    errors::RetailerError,
    structures::{HtmlRetailer, HtmlRetailerSuper, HtmlSearchQuery, Retailer},
    utils::{
        conversions::price_to_cents,
        html::{element_extract_attr, element_to_text},
    },
};

const SITEMAP: &str = "https://magdump.ca/sitemap";
const URL: &str = "https://magdump.ca/{category}?q=Availability-In+stock&from-xhr&page={page}";

#[derive(Deserialize)]
struct Response {
    products: Vec<ResponseProduct>,
    pagination: ResponsePagination,
}

impl Response {
    fn get_max_pages(&self) -> u64 {
        self.pagination.pages_count
    }
}

#[derive(Deserialize)]
struct ResponsePagination {
    pages_count: u64,
}

#[derive(Deserialize)]
struct ResponseProduct {
    add_to_cart_url: Option<String>,
    url: String,
    name: String,
    price_amount: f32,
    regular_price_amount: f32,
    cover: ResponseProductCover,
}

impl ResponseProduct {
    fn is_in_stock(&self) -> bool {
        self.add_to_cart_url.is_some()
    }

    fn get_name(&self) -> String {
        self.name.clone()
    }

    fn get_url(&self) -> String {
        self.url.clone()
    }

    fn get_price(&self) -> Result<Price, RetailerError> {
        let regular = price_to_cents(self.regular_price_amount.to_string())?;
        let sale = price_to_cents(self.price_amount.to_string())?;

        Ok(Price {
            regular_price: regular,
            sale_price: if regular != sale { Some(sale) } else { None },
        })
    }

    fn get_image(&self) -> String {
        self.cover.small.url.clone()
    }
}

#[derive(Deserialize)]
struct ResponseProductCover {
    small: ResponseProductCoverImage,
}

#[derive(Deserialize)]
struct ResponseProductCoverImage {
    url: String,
}

pub struct MagDump {
    crawler: UnprotectedCrawler,
    query: Vec<HtmlSearchQuery>,
}

impl Default for MagDump {
    fn default() -> Self {
        Self::new()
    }
}

impl MagDump {
    pub fn new() -> Self {
        Self {
            crawler: UnprotectedCrawler::new(),
            query: Vec::new(),
        }
    }

    fn init_get_uri(element: ElementRef) -> Result<String, RetailerError> {
        let link_href = element_extract_attr(element, "href")?;

        Ok(link_href.split_once(".ca/").unwrap().1.to_string())
    }
}

impl HtmlRetailerSuper for MagDump {}

#[async_trait]
impl Retailer for MagDump {
    // Marstar mixes and matches items in several categories
    // make an attempt to normalize the URIs
    async fn init(&mut self) -> Result<(), RetailerError> {
        let request = RequestBuilder::new().set_url(SITEMAP).build();
        let response = self.crawler.make_web_request(request).await?;

        let fragment = Html::parse_document(&response.body);
        let link_selector =
            Selector::parse("div.col-md-3 > ul > li > ul > li > a[id*='category-page']").unwrap();

        for link in fragment.select(&link_selector) {
            let link_name = element_to_text(link).to_lowercase();
            let uri = Self::init_get_uri(link)?;

            match link_name.as_str() {
                "rimfire" | "centerfire" | "bulk ammo" => {
                    self.query.push(HtmlSearchQuery {
                        term: uri,
                        category: Category::Ammunition,
                    });
                }
                "firearms" => {
                    self.query.push(HtmlSearchQuery {
                        term: uri,
                        category: Category::Firearm,
                    });
                }
                // handle the SBI category, there are firearms in here
                "sbi" => {
                    // TODO: deal with unwraps, this should be the parent <li> of the <a>
                    let parent = ElementRef::wrap(link.parent().unwrap()).unwrap();

                    let nested_selector =
                        Selector::parse("ul.nested > li > a[id*='category-page']").unwrap();

                    for sbi_child in parent.select(&nested_selector) {
                        let nested_text = element_to_text(sbi_child).to_lowercase();

                        if nested_text == "sbi" {
                            continue;
                        }

                        let nested_uri = Self::init_get_uri(sbi_child)?;

                        debug!("Parsing nested {nested_uri}");

                        self.query.push(HtmlSearchQuery {
                            term: nested_uri,
                            category: if nested_text == "rifles" {
                                Category::Firearm
                            } else {
                                Category::Other
                            },
                        });
                    }
                }
                "made in canada" | "airgun" => {}
                // I like playing games, add whatever we don't match as other
                _ => {
                    debug!("Matching non matched URL as other: {uri:?}");

                    self.query.push(HtmlSearchQuery {
                        term: uri,
                        category: Category::Other,
                    });
                }
            }
        }

        debug!("{:#?}", self.query);

        Ok(())
    }

    fn get_retailer_name(&self) -> RetailerName {
        RetailerName::MagDump
    }
}

#[async_trait]
impl HtmlRetailer for MagDump {
    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &HtmlSearchQuery,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", &(page_num + 1).to_string());

        debug!("Setting page to {}", url);

        let request = RequestBuilder::new()
            .set_url(url)
            .set_headers(&vec![(
                "Accept".to_string(),
                "application/json".to_string(),
            )])
            .build();

        Ok(request)
    }

    async fn parse_response(
        &self,
        response: &String,
        search_term: &HtmlSearchQuery,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let products = serde_json::from_str::<Response>(response)?;
        for product in products.products {
            if !product.is_in_stock() {
                continue;
            }

            let result = CrawlResult::new(
                product.get_name(),
                product.get_url(),
                product.get_price()?,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(product.get_image());

            results.push(result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<HtmlSearchQuery> {
        self.query.to_owned()
    }

    fn get_num_pages(&self, response: &String) -> Result<u64, RetailerError> {
        let products = serde_json::from_str::<Response>(response)?;

        Ok(products.get_max_pages())
    }
}
