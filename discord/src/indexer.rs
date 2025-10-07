use std::{cmp::max, collections::BTreeMap};

use common::{
    result::{
        base::CrawlResult,
        enums::{Category, RetailerName},
    },
    utils::get_current_time,
};
use retailers::errors::RetailerError;
use serenity::all::{CreateEmbed, EditWebhookMessage, ExecuteWebhook, MessageId};
use tokio::sync::OnceCell;
use tracing::debug;

use crate::client::DiscordClient;

static DISCORD_INDEXER_WEBHOOK: OnceCell<DiscordClient> = OnceCell::const_new();

#[cfg(not(debug_assertions))]
const INDEXER_WEBHOOK: &str = "https://discord.com/api/webhooks/1375013817091625032/2uqBwCzQGPbzHiHWvDBfY_xK7DyeFoyZ3WC40FxxwW1tD4EEDf2gYY3RzaM4vDYGiIbx";

#[cfg(debug_assertions)]
const INDEXER_WEBHOOK: &str = "https://discord.com/api/webhooks/1391665667987607592/qnLZbWGvfojAeLKUbspu59EMUxLL9aL8kkl76apvzl1oIk2vJ6VXYS0ZXF0pimlqUaQQ";

#[derive(Debug)]
struct RetailerStats {
    start_time: u64,
    end_time: Option<u64>,
    firearms_count: u64,
    // Total ammo count
    ammo_count: u64,
    // Total ammo with metadata (ie. round count)
    // ammo_count >= ammo_count_with_metadata
    ammo_count_with_metadata: u64,
    other_count: u64,
}

impl RetailerStats {
    fn new() -> Self {
        Self {
            start_time: get_current_time(),
            end_time: None,
            firearms_count: 0,
            ammo_count: 0,
            ammo_count_with_metadata: 0,
            other_count: 0,
        }
    }

    fn get_total_counts(&self) -> u64 {
        self.firearms_count + self.ammo_count + self.other_count
    }
}

pub struct IndexerWebhook {
    // BTreeMap is used over HashMap since BTreeMap sort themselves
    retailers: BTreeMap<RetailerName, RetailerStats>,
    main_message: Option<MessageId>,
}

impl IndexerWebhook {
    pub async fn new() -> Self {
        Self {
            retailers: BTreeMap::new(),
            main_message: None,
        }
    }

    async fn get_client() -> &'static DiscordClient {
        DISCORD_INDEXER_WEBHOOK
            .get_or_init(|| DiscordClient::new(INDEXER_WEBHOOK))
            .await
    }

    pub fn register_retailer(&mut self, retailer: RetailerName) {
        self.retailers.insert(retailer, RetailerStats::new());
    }

    pub async fn finish_retailer(&mut self, retailer: RetailerName, results: &Vec<&CrawlResult>) {
        let Some(retailer_stats) = self.retailers.get_mut(&retailer) else {
            return;
        };

        retailer_stats.end_time = Some(get_current_time());

        for result in results {
            match result.category {
                Category::Firearm => retailer_stats.firearms_count += 1,
                Category::Ammunition => {
                    retailer_stats.ammo_count += 1;

                    if result.metadata.is_some() {
                        retailer_stats.ammo_count_with_metadata += 1;
                    }
                }
                Category::Other => retailer_stats.other_count += 1,
                Category::_All => {}
            }
        }

        self.update_main_message().await;
    }

    pub async fn update_main_message(&mut self) {
        let mut messages: Vec<String> = Vec::new();

        for (retailer, stats) in &self.retailers {
            let counts = format!(
                "F: {} | O: {} | T: {}\nA: {}/{} ({:.2}%)",
                stats.firearms_count,
                stats.other_count,
                stats.get_total_counts(),
                stats.ammo_count_with_metadata,
                stats.ammo_count,
                100.0 * (stats.ammo_count_with_metadata as f32 / max(stats.ammo_count, 1) as f32)
            );

            messages.push(format!("{retailer:?}\n{counts}",));
        }

        let final_message = format!("```\n{}\n```", messages.join("\n\n"));

        let client = Self::get_client().await;

        if let Some(ref message) = self.main_message {
            debug!("Replaying message {}", message);

            let edit_builder = EditWebhookMessage::new().content(final_message);

            let _ = client
                .webhook
                .edit_message(client.http.clone(), *message, edit_builder)
                .await;
        } else {
            let builder = ExecuteWebhook::new().content(final_message);

            let result = client
                .webhook
                .execute(client.http.clone(), true, builder)
                .await
                .expect("Expected Discord API call to succeed")
                .expect("Expected message returned");

            debug!("No message set, setting to {}", result.id);
            self.main_message = Some(result.id);
        }
    }

    pub async fn send_message(&self, msg: String) {
        let message = format!("```{msg}```");
        let embed = CreateEmbed::new().description(message);
        let builder = ExecuteWebhook::new().embed(embed);

        let client = Self::get_client().await;

        let _ = client
            .webhook
            .execute(client.http.clone(), false, builder)
            .await;
    }

    pub async fn send_error(&self, name: RetailerName, err: RetailerError) {
        let message = format!("```{err}```");
        let embed = CreateEmbed::new()
            .title(format!("Error - {name:?}"))
            .description(message);
        let builder = ExecuteWebhook::new().embed(embed);

        let client = Self::get_client().await;

        let _ = client
            .webhook
            .execute(client.http.clone(), false, builder)
            .await;
    }
}
