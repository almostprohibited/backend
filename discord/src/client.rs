use std::sync::Arc;

use serenity::all::{Http, Webhook};

pub(crate) struct DiscordClient {
    pub(crate) http: Arc<Http>,
    pub(crate) webhook: Arc<Webhook>,
}

impl DiscordClient {
    pub(crate) async fn new(webhook_url: impl Into<String>) -> Self {
        let client = Arc::new(Http::new("this does not appear to matter"));
        let webhook = Arc::new(
            Webhook::from_url(&client, &webhook_url.into())
                .await
                .unwrap(),
        );

        Self {
            http: client.clone(),
            webhook: webhook.clone(),
        }
    }
}
