use std::{collections::BTreeMap, sync::Arc};

use chrono::{DateTime, FixedOffset, Offset, TimeZone};
use common::{
    result::{
        base::CrawlResult,
        enums::{Category, RetailerName},
    },
    utils::get_current_time,
};
use retailers::errors::RetailerError;
use serenity::all::{CreateEmbed, EditWebhookMessage, ExecuteWebhook, Http, MessageId, Webhook};
use tracing::debug;

#[cfg(not(debug_assertions))]
const INDEXER_WEBHOOK: &str = "https://discord.com/api/webhooks/1375013817091625032/2uqBwCzQGPbzHiHWvDBfY_xK7DyeFoyZ3WC40FxxwW1tD4EEDf2gYY3RzaM4vDYGiIbx";

#[cfg(debug_assertions)]
const INDEXER_WEBHOOK: &str = "https://discord.com/api/webhooks/1391665667987607592/qnLZbWGvfojAeLKUbspu59EMUxLL9aL8kkl76apvzl1oIk2vJ6VXYS0ZXF0pimlqUaQQ";

// west offset
const TZ_OFFSET: i32 = 7 * 3600;

#[derive(Debug)]
struct RetailerStats {
    start_time: u64,
    end_time: Option<u64>,
    firearms_count: u64,
    ammo_count: u64,
    other_count: u64,
}

impl RetailerStats {
    fn new() -> Self {
        Self {
            start_time: get_current_time(),
            end_time: None,
            firearms_count: 0,
            ammo_count: 0,
            other_count: 0,
        }
    }

    fn get_total_counts(&self) -> u64 {
        self.firearms_count + self.ammo_count + self.other_count
    }
}

fn timestamp_to_human_local(time: u64) -> DateTime<FixedOffset> {
    DateTime::from_timestamp(time as i64, 0)
        .expect("Creating DateTime should not fail until the year 292 million")
        .with_timezone(
            &FixedOffset::west_opt(TZ_OFFSET).expect("This should always be valid timezone"),
        )
}

pub struct IndexerWebhook {
    http: Arc<Http>,
    webhook: Arc<Webhook>,
    // BTreeMap is used over HashMap since BTreeMap sort themselves
    retailers: BTreeMap<RetailerName, RetailerStats>,
    main_message: Option<MessageId>,
}

impl IndexerWebhook {
    pub async fn new() -> Self {
        let client = Arc::new(Http::new("this does not appear to matter"));
        let webhook = Arc::new(Webhook::from_url(&client, INDEXER_WEBHOOK).await.unwrap());

        Self {
            http: client.clone(),
            webhook: webhook.clone(),
            retailers: BTreeMap::new(),
            main_message: None,
        }
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
                Category::Ammunition => retailer_stats.ammo_count += 1,
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
                "{} / {} / {} / {}",
                stats.firearms_count,
                stats.ammo_count,
                stats.other_count,
                stats.get_total_counts()
            );

            let end_time = match stats.end_time {
                Some(time) => timestamp_to_human_local(time).to_string(),
                None => "<running>".to_string(),
            };

            messages.push(format!(
                "{retailer:?}\n{} -> {end_time}\n{counts:<22}",
                timestamp_to_human_local(stats.start_time)
            ));
        }

        let final_message = format!("```\n{}\n```", messages.join("\n\n"));

        if let Some(ref message) = self.main_message {
            debug!("Replaying message {}", message);

            let edit_builder = EditWebhookMessage::new().content(final_message);

            let _ = self
                .webhook
                .edit_message(self.http.clone(), *message, edit_builder)
                .await;
        } else {
            let builder = ExecuteWebhook::new().content(final_message);

            let result = self
                .webhook
                .execute(self.http.clone(), true, builder)
                .await
                .expect("Expected Discord API call to succeed")
                .expect("Expected message returned");

            debug!("No message set, setting to {}", result.id);
            self.main_message = Some(result.id);
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
