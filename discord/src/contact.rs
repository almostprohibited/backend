use std::env;

use common::{constants::DISCORD_CONTACT_WEBHOOK_URL, messages::Message};
use serenity::all::CreateEmbed;
use tokio::sync::{Mutex, MutexGuard, OnceCell};

use crate::client::DiscordClient;

static DISCORD_CONTACT_WEBHOOK: OnceCell<Mutex<ContactWebhook>> = OnceCell::const_new();

pub struct ContactWebhook {
    client: DiscordClient,
}

impl ContactWebhook {
    async fn new() -> Self {
        // TODO: this fails when cell is populated, not during binary start
        // potentially causing ticking time bomb
        let webhook_env_var = env::var(DISCORD_CONTACT_WEBHOOK_URL).expect(&format!(
            "Expecting {DISCORD_CONTACT_WEBHOOK_URL} to be set"
        ));

        Self {
            client: DiscordClient::new(webhook_env_var).await,
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

        let _ = self.client.send_message(embed).await;
    }
}

pub async fn get_contact_webhook() -> MutexGuard<'static, ContactWebhook> {
    if !DISCORD_CONTACT_WEBHOOK.initialized() {
        let _ = DISCORD_CONTACT_WEBHOOK.set(Mutex::new(ContactWebhook::new().await));
    }

    DISCORD_CONTACT_WEBHOOK.get().unwrap().lock().await
}
