use std::sync::OnceLock;

use common::get_user_agent;
use openidconnect::reqwest::{Client, ClientBuilder, redirect::Policy};

static REQWEST_CLIENT: OnceLock<Client> = OnceLock::new();

pub(crate) fn get_reqwest_client() -> &'static Client {
    REQWEST_CLIENT.get_or_init(|| {
        // TODO: see if there is a way to not use vended client
        // this creates an openidconnect reqwest client
        // not the one I have in the workspace
        //
        // this did not work with native reqwest since
        // oidc needs some sort of async trait
        ClientBuilder::new()
            .redirect(Policy::none())
            .user_agent(get_user_agent())
            .https_only(true)
            .build()
            .expect("Valid client to be built")
    })
}
