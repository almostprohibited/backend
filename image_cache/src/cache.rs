use common::{
    domain_config::{ClientSettings, images::IMAGE_SETTINGS},
    image_cache::CachedImageObject,
    result::base::CrawlResult,
    string_utils::get_domain,
};
use crawler::{WebClient, errors::CrawlerError, request::RequestBuilder};
use tracing::debug;

use crate::{memory_cache::MemoryCache, traits::CacheMethod};

pub struct ImageCache {}

impl ImageCache {
    async fn download_image(url: &str) -> Result<CachedImageObject, CrawlerError> {
        let config = IMAGE_SETTINGS
            .get(&get_domain(url))
            .cloned()
            .unwrap_or(ClientSettings::default());

        let request = RequestBuilder::new()
            .set_url(url)
            .set_client_settings(config)
            .build();

        let crawler = WebClient::make_web_request(request).await?;

        let mime_type = crawler
            .headers
            .get("content-type")
            .expect("response to always have return type")
            .clone();

        Ok(CachedImageObject {
            mime_type,
            image: crawler.raw_bytes,
        })
    }

    // don't want to deal with providing my own missing image file
    // make the return type optional
    pub async fn get_image(crawl_result: CrawlResult) -> Option<CachedImageObject> {
        let image_url = crawl_result
            .image_url
            .clone()
            .expect("expecting image URL to always exist");

        if let Some(image) = MemoryCache::get_item(&image_url).await {
            debug!("Memory cache hit for {}", image_url);
            return Some(image);
        }

        if let Ok(downloaded_image) = Self::download_image(&image_url).await {
            debug!("Memory cache miss, downloading {}", image_url);

            MemoryCache::insert_item(&image_url, downloaded_image.clone()).await;

            return Some(downloaded_image);
        }

        return None;
    }
}
