use std::sync::Arc;

use serenity::all::{
    CreateEmbed, EditWebhookMessage, ExecuteWebhook, Http, Message, MessageId, Webhook,
};

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

    pub(crate) async fn send_message(
        &self,
        embeds: Vec<CreateEmbed>,
    ) -> Result<Option<Message>, serenity::Error> {
        let builder = ExecuteWebhook::new().embeds(embeds);

        self.webhook.execute(self.http.clone(), true, builder).await
    }

    pub(crate) async fn update_message(
        &self,
        message_id: MessageId,
        embeds: Vec<CreateEmbed>,
    ) -> Result<Message, serenity::Error> {
        let builder = EditWebhookMessage::new().embeds(embeds);

        self.webhook
            .edit_message(self.http.clone(), message_id, builder)
            .await
    }
}
