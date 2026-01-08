use std::{num::NonZeroUsize, sync::LazyLock};

use common::image_cache::CachedImageObject;
use lru::LruCache;
use tokio::sync::Mutex;

use crate::traits::CacheMethod;

const MAX_CACHE_SIZE: usize = 1000;

static CACHE: LazyLock<Mutex<LruCache<String, CachedImageObject>>> =
    LazyLock::new(|| Mutex::new(LruCache::new(NonZeroUsize::new(MAX_CACHE_SIZE).unwrap())));

pub(crate) struct MemoryCache {}

impl CacheMethod for MemoryCache {
    async fn get_item(cache_key: &str) -> Option<CachedImageObject> {
        CACHE.lock().await.get(cache_key).cloned()
    }

    async fn insert_item(cache_key: &str, image: CachedImageObject) {
        CACHE.lock().await.push(cache_key.to_string(), image);
    }
}
