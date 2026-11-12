use std::env;

use common::{
    constants::{
        PROXY_NORMAL_ADDRESS, PROXY_NORMAL_PASSWORD, PROXY_NORMAL_USERNAME,
        PROXY_RESIDENTIAL_ADDRESS, PROXY_RESIDENTIAL_PASSWORD, PROXY_RESIDENTIAL_USERNAME,
    },
    domain_config::ProxyType,
};

pub(crate) struct ProxyDetails {
    pub(crate) address: String,
    pub(crate) username: Option<String>,
    pub(crate) password: Option<String>,
}

pub(crate) fn get_proxy_details(proxy: ProxyType) -> Option<ProxyDetails> {
    let (address, username, password) = match proxy {
        ProxyType::Normal => {
            let addr = env::var(PROXY_NORMAL_ADDRESS)
                .expect(&format!("Expected {PROXY_NORMAL_ADDRESS} to be populated"));
            let user = env::var(PROXY_NORMAL_USERNAME).ok();
            let pass = env::var(PROXY_NORMAL_PASSWORD).ok();

            (addr, user, pass)
        }
        ProxyType::Residential => {
            let addr = env::var(PROXY_RESIDENTIAL_ADDRESS).expect(&format!(
                "Expected {PROXY_RESIDENTIAL_ADDRESS} to be populated"
            ));
            let user = env::var(PROXY_RESIDENTIAL_USERNAME).ok();
            let pass = env::var(PROXY_RESIDENTIAL_PASSWORD).ok();

            (addr, user, pass)
        }
        _ => return None,
    };

    return Some(ProxyDetails {
        address,
        username,
        password,
    });
}
