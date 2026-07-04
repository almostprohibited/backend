use std::hash::{DefaultHasher, Hash, Hasher};

use chrono::{Datelike, Utc};

const USER_AGENTS: [&str; 7] = [
    "Mozilla/5.0 (Linux; Android 10; K) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/149.0.0.0 Mobile Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/150.0.0.0 Safari/537.36",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 18_7 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Mobile/15E148",
    "Mozilla/5.0 (iPhone; CPU iPhone OS 18_7 like Mac OS X) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.7.5 Mobile/15E148 Safari/604.1",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:151.0) Gecko/20100101 Firefox/151.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/26.5 Safari/605.1.15",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:152.0) Gecko/20100101 Firefox/152.0",
];

// sorry to any retailers that have been caught up in this
// if you end up on here, you were contacted by email and
// should know about this
const RETAILER_HOSTS: [&str; 1] = ["reliablegun.com"];

fn hash_host_to_index(host: &str) -> u64 {
    let datetime = Utc::now();

    let mut hasher = DefaultHasher::new();

    host.hash(&mut hasher);
    datetime.day().hash(&mut hasher);
    datetime.month().hash(&mut hasher);
    datetime.year().hash(&mut hasher);

    hasher.finish() % USER_AGENTS.len() as u64
}

pub(crate) fn shuffle_user_agent(url: &str) -> Option<String> {
    let Some(host) = RETAILER_HOSTS.iter().find(|host| url.contains(*host)) else {
        return None;
    };

    let index = hash_host_to_index(host);

    let Some(user_agent) = USER_AGENTS.get(index as usize) else {
        return Some(USER_AGENTS[0].to_string());
    };

    Some(user_agent.to_string())
}
