use std::{collections::HashMap, sync::LazyLock};

use crate::domain_config::{ClientSettings, ClientType, ProxyType};

pub static DEFAULT_SETTINGS: LazyLock<HashMap<String, ClientSettings>> = LazyLock::new(|| {
    HashMap::from_iter(vec![
        (
            "italiansportinggoods.com".into(), // canadian IP check
            ClientSettings::default().with_proxy_type(ProxyType::Normal),
        ),
        // (
        //     "x-reload.com".into(), // canadian IP check
        //     ClientSettings::default().with_proxy_type(ProxyType::Normal),
        // ),
        (
            "dantesports.com".into(), // canadian IP check
            ClientSettings::default().with_proxy_type(ProxyType::Normal),
        ),
        (
            "londerosports.com".into(), // odd behaviour, they protect their images with CF but not products
            ClientSettings::default()
                .with_proxy_type(ProxyType::Normal)
                .without_http_sigs(),
        ),
        (
            "internationalshootingsupplies.com".into(), // canadian IP check
            ClientSettings::default()
                .with_proxy_type(ProxyType::Normal)
                .without_http_sigs(),
        ),
        (
            "thegundealer.ca".into(), // canadian IP check
            ClientSettings::default()
                .with_proxy_type(ProxyType::Normal)
                .without_http_sigs(),
        ),
        (
            "swampdonkeyoutdoors.ca".into(), // canadian IP check
            ClientSettings::default()
                .with_proxy_type(ProxyType::Normal)
                .without_http_sigs(),
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
        (
            "rangeviewsports.ca".into(), // cloudflare bot checks
            ClientSettings::default()
                .with_proxy_type(ProxyType::Normal)
                .with_client_type(ClientType::Emulated)
                .without_http_sigs(),
        ),
        (
            "aagcanada.ca".into(), // shopify
            ClientSettings::default()
                .with_proxy_type(ProxyType::Normal)
                .with_client_type(ClientType::Emulated)
                .without_http_sigs(),
        ),
        (
            "fishingworldgc.ca".into(), // shopify
            ClientSettings::default()
                .with_proxy_type(ProxyType::Normal)
                .with_client_type(ClientType::Emulated)
                .without_http_sigs(),
        ),
        (
            "uxbridgearms.com".into(), // shopify
            ClientSettings::default()
                .with_proxy_type(ProxyType::Normal)
                .with_client_type(ClientType::Emulated)
                .without_http_sigs(),
        ),
        (
            "crafm.com".into(), // shopify
            ClientSettings::default()
                .with_proxy_type(ProxyType::Normal)
                .with_client_type(ClientType::Emulated)
                .without_http_sigs(),
        ),
        (
            "intersurplus.com".into(), // shopify
            ClientSettings::default()
                .with_proxy_type(ProxyType::Normal)
                .with_client_type(ClientType::Emulated)
                .without_http_sigs(),
        ),
    ])
});
