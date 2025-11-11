use crawler::{request::RequestBuilder, unprotected::UnprotectedCrawler};
use scraper::{Html, Selector};

use crate::{
    errors::RetailerError,
    structures::HtmlSearchQuery,
    utils::{
        ecommerce::{BigCommerce, bigcommerce::structs::SitemapEntry},
        html::{element_extract_attr, element_to_text},
    },
};

pub(crate) trait BigCommerceSitemap {
    async fn get_search_terms<T: Fn(SitemapEntry) -> Option<HtmlSearchQuery>>(
        base_url: impl Into<String>,
        filter_map_method: T,
    ) -> Result<Vec<HtmlSearchQuery>, RetailerError>;
}

impl BigCommerceSitemap for BigCommerce {
    async fn get_search_terms<T: Fn(SitemapEntry) -> Option<HtmlSearchQuery>>(
        base_url: impl Into<String>,
        filter_map_method: T,
    ) -> Result<Vec<HtmlSearchQuery>, RetailerError> {
        let mut parsed_base_url = base_url.into();
        if !parsed_base_url.ends_with("/") {
            parsed_base_url += "/";
        }

        let sitemap_url = format!("{parsed_base_url}/sitemap/categories");

        let request = RequestBuilder::new().set_url(sitemap_url).build();
        let response = UnprotectedCrawler::make_web_request(request).await?;

        let sitemap = Html::parse_fragment(&response.body);
        let selector = Selector::parse("div.container > ul > li li > a").unwrap();

        let links: Vec<HtmlSearchQuery> = sitemap
            .select(&selector)
            .filter_map(|el| {
                let category_name = element_to_text(el);

                let Ok(mut category_url) = element_extract_attr(el, "href") else {
                    // TODO: probably should log this error somewhere
                    // don't think this will happen though
                    return None;
                };

                category_url = category_url.replace(&parsed_base_url, "");

                if category_url.ends_with("/") {
                    category_url.pop();
                }

                Some(SitemapEntry {
                    name: category_name,
                    part: category_url,
                })
            })
            .filter_map(filter_map_method)
            .collect::<Vec<HtmlSearchQuery>>();

        Ok(links)
    }
}
