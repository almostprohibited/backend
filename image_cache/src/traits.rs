use common::image_cache::CachedImageObject;

pub(crate) trait CacheMethod {
    fn get_item(cache_key: &str) -> impl Future<Output = Option<CachedImageObject>>;
    fn insert_item(cache_key: &str, image: CachedImageObject) -> impl Future<Output = ()>;
}
