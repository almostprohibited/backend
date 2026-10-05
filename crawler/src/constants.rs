pub(crate) const PAGE_TIMEOUT_SECONDS: u64 = 60;

pub(crate) const PROXY_DOMAINS: [&str; 7] = [
    "italiansportinggoods.com",
    "x-reload.com",
    "dantesports.com",
    "londerosports.com",
    "internationalshootingsupplies.com",
    "thegundealer.ca",
    "swampdonkeyoutdoors.ca",
];

pub(crate) const EMULATED_DOMAINS: [&str; 7] = [
    "reliablegun.com",
    "rangeviewsports.ca",
    // shopify appears to completely
    // gimp requests that come from a datacenter
    // and have a "non-human" UA regardless of
    // web bot signed headers
    "aagcanada.ca",      // shopify
    "fishingworldgc.ca", // shopify
    "uxbridgearms.com",  // shopify
    "crafm.com",         // shopify
    "intersurplus.com",  // shopify
];

// TODO: this messes with TLS/HTTP fingerprinting
// careful when adding domains here
/// Add domains here that should NOT have web sigs generated
pub(crate) const HTTP_SIG_EXCLUDED_DOMAINS: [&str; 3] = [
    "londerosports.com",
    "reliablegun.com",
    "rangeviewsports.ca",
    // "aagcanada.ca",      // shopify
    // "fishingworldgc.ca", // shopify
    // "uxbridgearms.com",  // shopify
    // "crafm.com",         // shopify
    // "intersurplus.com",  // shopify
];
