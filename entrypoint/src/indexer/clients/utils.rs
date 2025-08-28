use common::result::{base::CrawlResult, enums::Category};

/// Responsible for determining ordering of duplicated entries.
/// For example, if we found the same product in the firearm and ammo category,
/// then the item is probably ammo, not a firearm
pub(crate) fn get_category_tier(category: Category) -> i64 {
    match category {
        Category::Firearm => 0,
        Category::Ammunition => 1,
        Category::Other => 2,
        _ => -1,
    }
}

/// Creates a "unique" key for the results hashing to dedupe products
pub(crate) fn get_key(crawl_result: &CrawlResult) -> String {
    format!("{}{}", crawl_result.name, crawl_result.url)
}
