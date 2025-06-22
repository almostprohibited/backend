use async_trait::async_trait;
use common::result::{
    base::{CrawlResult, Price},
    enums::{Category, RetailerName},
};
use crawler::{request::Request, unprotected::UnprotectedCrawler};
use scraper::{ElementRef, Html, Selector};
use tracing::{debug, error};

use crate::{
    errors::RetailerError,
    traits::{Retailer, SearchTerm},
    utils::{
        conversions::{price_to_cents, string_to_u64},
        html::{element_extract_attr, element_to_text, extract_element_from_element},
    },
};

const CRAWL_DELAY_SECS: u64 = 10; // https://www.reliablegun.com/robots.txt
const PAGE_SIZE: u64 = 24; // Reliable Gun's site is slow
const BASE_URL: &str = "https://www.reliablegun.com";
const URL: &str = "https://www.reliablegun.com/{category}#/pageSize={page_size}&viewMode=grid&orderBy=0&pageNumber={page}";

pub struct ReliableGun {
    crawler: UnprotectedCrawler,
    retailer: RetailerName,
}

impl ReliableGun {
    pub fn new() -> ReliableGun {
        ReliableGun {
            crawler: UnprotectedCrawler::new(),
            retailer: RetailerName::ReliableGun,
        }
    }

    fn find_prices(element: ElementRef) -> Result<Price, RetailerError> {
        let actual_element = extract_element_from_element(element, "span.actual-price")?;
        let actual_price = price_to_cents(element_to_text(actual_element))?;

        let mut price = Price {
            regular_price: actual_price,
            sale_price: None,
        };

        if let Ok(old_price_element) = extract_element_from_element(element, "span.old-price") {
            let old_price = price_to_cents(element_to_text(old_price_element))?;

            price.sale_price = Some(price.regular_price);
            price.regular_price = old_price;
        }

        Ok(price)
    }

    fn extract_page_num_from_href(element: ElementRef) -> Result<u64, RetailerError> {
        let href = element_extract_attr(element, "href")?;

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
    fn get_retailer_name(&self) -> RetailerName {
        self.retailer
    }

    async fn build_page_request(
        &self,
        page_num: u64,
        search_term: &SearchTerm,
    ) -> Result<Request, RetailerError> {
        let url = URL
            .replace("{category}", &search_term.term)
            .replace("{page}", (page_num + 1).to_string().as_str())
            .replace("{page_size}", PAGE_SIZE.to_string().as_str());

        debug!("Setting URL to {}", URL);

        let request_builder = Request::builder()
            .set_url(url)
            .set_headers(&[("User-Agent".into(), "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/135.0.0.0 Safari/537.36".into())].to_vec());

        Ok(request_builder.build())
    }

    async fn parse_response(
        &self,
        response: &String,
        search_term: &SearchTerm,
    ) -> Result<Vec<CrawlResult>, RetailerError> {
        let mut results: Vec<CrawlResult> = Vec::new();

        let fragment = Html::parse_document(response);

        for element in fragment.select(&Selector::parse("div.product-item").unwrap()) {
            let description_element = extract_element_from_element(element, "div.description")?;
            let url_element = extract_element_from_element(element, "h2.product-title > a")?;
            let image_element = extract_element_from_element(element, "img.product-overview-img")?;

            let description = element_to_text(description_element);
            let url_href = element_extract_attr(url_element, "href")?;
            let name = element_to_text(url_element);
            let image_url = element_extract_attr(image_element, "src")?;

            let price = Self::find_prices(element)?;

            let new_result = CrawlResult::new(
                name,
                format!("{}{}", BASE_URL, url_href),
                price,
                self.get_retailer_name(),
                search_term.category,
            )
            .with_image_url(image_url.to_string())
            .with_description(description);

            results.push(new_result);
        }

        Ok(results)
    }

    fn get_search_terms(&self) -> Vec<SearchTerm> {
        Vec::from_iter([
            SearchTerm {
                term: "firearms".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "used-guns-non-restricted".into(),
                category: Category::Firearm,
            },
            SearchTerm {
                term: "optics".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "reloading".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "used-optics".into(),
                category: Category::Other,
            },
            SearchTerm {
                term: "accessories".into(),
                category: Category::Other,
            },
        ])
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
