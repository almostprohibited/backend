use std::{collections::HashMap, sync::LazyLock};

use common::image_cache::CachedImageObject;
use tokio::sync::Mutex;

use crate::traits::CacheMethod;

static CACHE: LazyLock<Mutex<HashMap<String, CachedImageObject>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub(crate) struct MemoryCache {}

impl CacheMethod for MemoryCache {
    async fn get_item(cache_key: &str) -> Option<CachedImageObject> {
        CACHE.lock().await.get(cache_key).cloned()
    }

    async fn insert_item(cache_key: &str, image: CachedImageObject) {
        CACHE.lock().await.insert(cache_key.to_string(), image);
    }
}
