use std::{cmp::max, collections::BTreeMap, env};

use common::{
    constants::DISCORD_INDEXER_WEBHOOK_URL,
    result::{
        base::CrawlResult,
        enums::{Category, RetailerName},
    },
    utils::get_current_time,
};
use serenity::all::{Colour, CreateEmbed, MessageId};
use tokio::sync::{Mutex, MutexGuard, OnceCell};

use crate::client::DiscordClient;

static DISCORD_INDEXER_WEBHOOK: OnceCell<Mutex<IndexerWebhook>> = OnceCell::const_new();

enum IndexingState {
    InProgress,
    InProgressError,
    FinishedSuccess,
    FinishedError,
}

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
    errors: Vec<String>,
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
            errors: Vec::new(),
        }
    }

    fn get_total_counts(&self) -> u64 {
        self.firearms_count + self.ammo_count + self.other_count
    }
}

pub struct IndexerWebhook {
    client: DiscordClient,
    // BTreeMap is used over HashMap since BTreeMap sort themselves
    retailers: BTreeMap<RetailerName, RetailerStats>,
    main_message: Option<MessageId>,
    state: IndexingState,
}

impl IndexerWebhook {
    pub async fn new() -> Self {
        // TODO: this fails when cell is populated, not during binary start
        // potentially causing ticking time bomb
        let webhook_env_var = env::var(DISCORD_INDEXER_WEBHOOK_URL)
            .unwrap_or_else(|_| panic!("Expecting {DISCORD_INDEXER_WEBHOOK_URL} to be set"));

        Self {
            client: DiscordClient::new(webhook_env_var).await,
            retailers: BTreeMap::new(),
            main_message: None,
            state: IndexingState::InProgress,
        }
    }

    pub fn register_retailer(&mut self, retailer: RetailerName) {
        self.retailers.insert(retailer, RetailerStats::new());
    }

    pub fn record_retailer_failure(&mut self, retailer: RetailerName, error: impl Into<String>) {
        let Some(retailer_stats) = self.retailers.get_mut(&retailer) else {
            return;
        };

        retailer_stats.errors.push(error.into());
    }

    pub fn append_retailer_stats(&mut self, retailer: RetailerName, results: &Vec<&CrawlResult>) {
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
    }

    fn get_embed_colour(&self) -> Colour {
        match self.state {
            IndexingState::InProgress => Colour::from_rgb(35, 127, 235), // blue
            IndexingState::InProgressError => Colour::from_rgb(235, 143, 35), // orange
            IndexingState::FinishedSuccess => Colour::from_rgb(35, 235, 143), // green
            IndexingState::FinishedError => Colour::from_rgb(235, 35, 127), // pink?
        }
    }

    // I don't like making this mutable, but whatever, ops tooling
    fn create_indexer_report_embed(&mut self) -> Vec<CreateEmbed> {
        let mut embeds: Vec<CreateEmbed> = Vec::new();
        let mut fields: Vec<(String, String, bool)> = Vec::new();

        let mut count: u64 = 0;

        // TODO: deal with splitting embeds properly
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

            let mut retailer_field: String = format!("```\n{counts}\n```");

            if !stats.errors.is_empty() {
                if matches!(self.state, IndexingState::InProgress) {
                    self.state = IndexingState::InProgressError;
                }

                let error_blob = stats.errors.join("\n");
                retailer_field += &format!("**```\n{error_blob}\n```**");
            }

            fields.push((retailer.to_string(), retailer_field, false));

            count += 1;

            if count % 25 == 0 {
                embeds.push(
                    CreateEmbed::new()
                        .fields(fields.clone())
                        .colour(self.get_embed_colour()),
                );

                fields.clear();
            }
        }

        if !fields.is_empty() {
            embeds.push(
                CreateEmbed::new()
                    .fields(fields)
                    .colour(self.get_embed_colour()),
            );
        }

        embeds
    }

    pub async fn update_main_message(&mut self) {
        let embeds = self.create_indexer_report_embed();

        if let Some(main_message) = self.main_message {
            let _ = self.client.update_message(main_message, embeds).await;
        } else {
            let returned_message_id = self
                .client
                .send_message(embeds)
                .await
                .expect("Expected Discord API call to succeed");

            self.main_message = Some(returned_message_id.expect("Expected message returned").id);
        }
    }

    pub fn finish(&mut self) {
        self.state = match self.state {
            IndexingState::InProgressError => IndexingState::FinishedError,
            _ => IndexingState::FinishedSuccess,
        };
    }
}

pub async fn get_indexer_webhook() -> MutexGuard<'static, IndexerWebhook> {
    if !DISCORD_INDEXER_WEBHOOK.initialized() {
        let _ = DISCORD_INDEXER_WEBHOOK.set(Mutex::new(IndexerWebhook::new().await));
    }

    DISCORD_INDEXER_WEBHOOK.get().unwrap().lock().await
}
