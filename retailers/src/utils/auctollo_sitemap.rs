use crawler::{request::RequestBuilder, unprotected::UnprotectedCrawler};
use scraper::{Html, Selector};

use crate::{errors::RetailerError, structures::HtmlSearchQuery, utils::html::element_to_text};

pub(crate) async fn get_search_queries<T: Fn(String) -> Option<HtmlSearchQuery>>(
    sitemap_url: impl Into<String>,
    product_url_base: &str,
    filter_map_method: T,
) -> Result<Vec<HtmlSearchQuery>, RetailerError> {
    let crawler = UnprotectedCrawler::new();
    let request = RequestBuilder::new().set_url(sitemap_url).build();
    let response = crawler.make_web_request(request).await?;

    let sitemap = Html::parse_fragment(&response.body);
    let selector = Selector::parse("urlset > url > loc").unwrap();
    let links: Vec<HtmlSearchQuery> = sitemap
        .select(&selector)
        .map(|el| {
            let mut cleaned_text = element_to_text(el).replace(product_url_base, "");

            if cleaned_text.ends_with("/") {
                cleaned_text.pop();
            }

            cleaned_text
        })
        .filter_map(filter_map_method)
        .collect::<Vec<HtmlSearchQuery>>();

    Ok(links)
}
