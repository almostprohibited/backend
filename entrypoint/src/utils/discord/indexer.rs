use std::sync::Arc;

use common::result::enums::RetailerName;
use retailers::errors::RetailerError;
use serenity::all::{CreateEmbed, ExecuteWebhook, Http, Webhook};

const INDEXER_WEBHOOK: &str = "https://discord.com/api/webhooks/1375013817091625032/2uqBwCzQGPbzHiHWvDBfY_xK7DyeFoyZ3WC40FxxwW1tD4EEDf2gYY3RzaM4vDYGiIbx";

pub struct IndexerWebhook {
    http: Arc<Http>,
    webhook: Arc<Webhook>,
}

impl IndexerWebhook {
    pub async fn new() -> Self {
        let client = Arc::new(Http::new("this does not appear to matter"));
        let webhook = Arc::new(Webhook::from_url(&client, INDEXER_WEBHOOK).await.unwrap());

        Self {
            http: client.clone(),
            webhook: webhook.clone(),
        }
    }

    pub async fn send_message(&self, msg: String) {
        let message = format!("```{}```", msg);
        let embed = CreateEmbed::new().description(message);
        let builder = ExecuteWebhook::new().embed(embed);

        let _ = self
            .webhook
            .execute(self.http.clone(), false, builder)
            .await;
    }

    pub async fn send_error(&self, name: RetailerName, err: RetailerError) {
        let message = format!("```{}```", err);
        let embed = CreateEmbed::new()
            .title(format!("Error - {:?}", name))
            .description(message);
        let builder = ExecuteWebhook::new().embed(embed);

        let _ = self
            .webhook
            .execute(self.http.clone(), false, builder)
            .await;
    }
}
