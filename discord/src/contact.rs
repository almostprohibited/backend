use common::messages::Message;
use serenity::all::{CreateEmbed, ExecuteWebhook};
use tokio::sync::OnceCell;

use crate::client::DiscordClient;

static DISCORD_CONTACT_WEBHOOK: OnceCell<DiscordClient> = OnceCell::const_new();

const CONTACT_WEBHOOK: &str = "https://discord.com/api/webhooks/1383689431592210462/LszB63q-H2y7HiNObCDxqv8YpTRRWvRk9FPF3qqIp201bZIJoNijzm1ZcxgWGIjFxqmx";

pub struct ContactWebhook {}

impl ContactWebhook {
    async fn get_client() -> &'static DiscordClient {
        DISCORD_CONTACT_WEBHOOK
            .get_or_init(|| DiscordClient::new(CONTACT_WEBHOOK))
            .await
    }

    pub async fn relay_message(message: Message) {
        let embed = CreateEmbed::new().title("New message").fields([
            ("IP address", message.ip_address, false),
            ("Time", format!("<t:{}:R>", message.timestamp), false),
            ("Email", message.email.unwrap_or("null".into()), false),
            ("Subject", message.subject.unwrap_or("null".into()), false),
            ("Body", message.body, false),
        ]);

        let builder = ExecuteWebhook::new().embed(embed);

        let client = Self::get_client().await;

        let _ = client
            .webhook
            .execute(client.http.clone(), false, builder)
            .await;
    }
}
