use std::sync::Arc;

use common::messages::Message;
use serenity::all::{CreateEmbed, ExecuteWebhook, Http, Webhook};

const CONTACT_WEBHOOK: &str = "https://discord.com/api/webhooks/1383689431592210462/LszB63q-H2y7HiNObCDxqv8YpTRRWvRk9FPF3qqIp201bZIJoNijzm1ZcxgWGIjFxqmx";

pub struct ContactWebhook {
    http: Arc<Http>,
    webhook: Arc<Webhook>,
}

impl ContactWebhook {
    pub async fn new() -> Self {
        let client = Arc::new(Http::new("this does not appear to matter"));
        let webhook = Arc::new(Webhook::from_url(&client, CONTACT_WEBHOOK).await.unwrap());

        Self {
            http: client.clone(),
            webhook: webhook.clone(),
        }
    }

    pub async fn relay_message(&self, message: Message) {
        let embed = CreateEmbed::new().title("New message").fields([
            ("IP address", message.ip_address, false),
            ("Time", format!("<t:{}:R>", message.timestamp), false),
            ("Email", message.email.unwrap_or("null".into()), false),
            ("Subject", message.subject.unwrap_or("null".into()), false),
            ("Body", message.body, false),
        ]);

        let builder = ExecuteWebhook::new().embed(embed);

        let _ = self
            .webhook
            .execute(self.http.clone(), false, builder)
            .await;
    }
}
