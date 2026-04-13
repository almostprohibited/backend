/// Strips URL params
pub(crate) fn clean_url(url: &str) -> String {
    match url.split_once("?") {
        Some((clean_url, _)) => clean_url.to_string(),
        None => url.to_string(),
    }
}
