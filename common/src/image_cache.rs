use reqwest::header::HeaderValue;

#[derive(Clone)]
pub struct CachedImageObject {
    pub mime_type: HeaderValue,
    pub image: Vec<u8>,
}
