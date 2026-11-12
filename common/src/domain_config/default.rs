use std::{collections::HashMap, sync::LazyLock};

use crate::domain_config::{ClientSettings, ClientType, ProxyType};

pub static DEFAULT_SETTINGS: LazyLock<HashMap<String, ClientSettings>> = LazyLock::new(|| {
    HashMap::from_iter(vec![
        (
            "italiansportinggoods.com".into(), // canadian IP check
            ClientSettings::default().with_proxy_type(ProxyType::Normal),
        ),
        // (
        //     "dantesports.com".into(), // canadian IP check
        //     ClientSettings::default().with_proxy_type(ProxyType::Normal),
        // ),
        (
            "londerosports.com".into(), // canadian IP check, they also hate the http sig for some reason
            ClientSettings::default()
                .with_proxy_type(ProxyType::Residential)
                .without_http_sigs(),
        ),
        (
            "internationalshootingsupplies.com".into(), // canadian IP check
            ClientSettings::default().with_proxy_type(ProxyType::Normal),
        ),
        // (
        //     "thegundealer.ca".into(), // canadian IP check
        //     ClientSettings::default().with_proxy_type(ProxyType::Normal),
        // ),
        (
            "swampdonkeyoutdoors.ca".into(), // canadian IP check
            ClientSettings::default().with_proxy_type(ProxyType::Normal),
        ),
        (
            "victoryridgesports.ca".into(), // canadian IP check
            ClientSettings::default().with_proxy_type(ProxyType::Normal),
        ),
        (
            "reliablegun.com".into(), // we are blocked by user agent, no email response
            ClientSettings::default()
                .with_proxy_type(ProxyType::Normal)
                .with_client_type(ClientType::Emulated)
                .without_http_sigs(),
        ),
        // (
        //     "rangeviewsports.ca".into(), // cloudflare bot checks
        //     ClientSettings::default()
        //         .with_proxy_type(ProxyType::Normal)
        //         .with_client_type(ClientType::Emulated)
        //         .without_http_sigs(),
        // ),
        // (
        //     "latulippe.com".into(), // cloudflare bot checks
        //     ClientSettings::default()
        //         .with_client_type(ClientType::Browser)
        //         .without_http_sigs(),
        // ),
        (
            "aagcanada.ca".into(), // shopify
            ClientSettings::default().with_proxy_type(ProxyType::Normal),
        ),
        (
            "fishingworldgc.ca".into(), // shopify
            ClientSettings::default().with_proxy_type(ProxyType::Normal),
        ),
        (
            "uxbridgearms.com".into(), // shopify
            ClientSettings::default().with_proxy_type(ProxyType::Normal),
        ),
        (
            "crafm.com".into(), // shopify
            ClientSettings::default().with_proxy_type(ProxyType::Normal),
        ),
        (
            "intersurplus.com".into(), // shopify
            ClientSettings::default().with_proxy_type(ProxyType::Normal),
        ),
    ])
});
