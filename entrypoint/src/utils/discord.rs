use std::sync::Arc;

use retailers::errors::RetailerError;
use serenity::all::{CreateEmbed, ExecuteWebhook, Http, Webhook};

const URL: &str = "https://discord.com/api/webhooks/1375013817091625032/2uqBwCzQGPbzHiHWvDBfY_xK7DyeFoyZ3WC40FxxwW1tD4EEDf2gYY3RzaM4vDYGiIbx";

pub struct Discord {
    http: Arc<Http>,
    webhook: Arc<Webhook>,
}

impl Discord {
    pub async fn new() -> Self {
        let client = Arc::new(Http::new("this does not appear to matter"));
        let webhook = Arc::new(Webhook::from_url(&client, URL).await.unwrap());

        Self {
            http: client.clone(),
            webhook: webhook.clone(),
        }
    }

    pub async fn test(&self) {
        let embed = CreateEmbed::new().title("Title").description("description");
        let builder = ExecuteWebhook::new().embed(embed);

        let _ = self
            .webhook
            .execute(self.http.clone(), false, builder)
            .await;
    }

    pub async fn send_error(&self, err: RetailerError) {
        let message = format!("```{}```", err);
        let embed = CreateEmbed::new().title("Title").description(message);
        let builder = ExecuteWebhook::new().embed(embed);

        let _ = self
            .webhook
            .execute(self.http.clone(), false, builder)
            .await;
    }
}
