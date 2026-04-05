pub(crate) const DATABASE_NAME: &str = "project-carbon";

pub(crate) const COLLECTION_CRAWL_RESULTS_NAME: &str = "crawl-results";

pub(crate) const VIEW_LIVE_DATA_NAME: &str = "live-results";
pub(crate) const VIEW_LIVE_DATA_SEARCH_INDEX: &str = "name_text";

pub(crate) const COLLECTION_MESSAGES_NAME: &str = "messages";

pub(crate) const COLLECTION_PRICE_HISTORY_NAME: &str = "price-history";

pub(crate) const COLLECTION_USERS_NAME: &str = "users";
pub(crate) const COLLECTION_SESSIONS_NAME: &str = "sessions";
pub(crate) const SESSIONS_EXPIRE_INDEX: &str = "session_expire_index";
pub(crate) const COLLECTION_VERIFICATIONS_NAME: &str = "verifications";
pub(crate) const VERIFICATIONS_EXPIRE_INDEX: &str = "verifications_expire_index";

/// 30 mins
pub(crate) const VERIFICATIONS_EXPIRY_SECONDS: u64 = 60 * 30;
