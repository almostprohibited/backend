use clap::Parser;
use common::result::enums::RetailerName;
use metrics::_private::PROVIDER;
use mongodb_connector::connector::MongoDBConnector;
use std::{sync::Arc, time::Duration};
use tokio::{sync::Mutex, task::JoinHandle, time::sleep};
use tracing::{debug, info};
use utils::{discord::indexer::IndexerWebhook, logger::configure_logger};

use crate::retailers::get_retailers;

mod clients;
mod retailers;

// https://nickb.dev/blog/default-musl-allocator-considered-harmful-to-performance
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: mimalloc::MiMalloc = mimalloc::MiMalloc;

#[derive(Parser)]
#[command(version)]
struct Arguments {
    /// List of retailers to crawl, crawls all retailers by default
    #[arg(short, long, value_delimiter = ' ', num_args = 0..)]
    retailers: Vec<RetailerName>,
    /// List of retailers to exclude from crawling
    #[arg(short, long, value_delimiter = ' ', num_args = 0..)]
    excluded_retailers: Vec<RetailerName>,
    /// Does not write to DB if set
    #[arg(short, long, default_value_t = false)]
    dry_run: bool,
}

#[tokio::main]
async fn main() {
    let args = Arguments::parse();

    configure_logger();

    let discord_webhook = Arc::new(Mutex::new(IndexerWebhook::new().await));

    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    let mongodb = Arc::new(MongoDBConnector::new().await);

    for mut retailer in get_retailers(args.retailers, args.excluded_retailers) {
        let db = mongodb.clone();
        let discord_webhook = discord_webhook.clone();

        handles.push(tokio::spawn(async move {
            let retailer_name = retailer.get_retailer_name().clone();
            info!("Registering {retailer_name:?}");

            discord_webhook
                .lock()
                .await
                .register_retailer(retailer_name);

            let crawl_state = retailer.crawl().await;
            let results = retailer.get_results();

            debug!("{:?}", results);

            if let Err(err) = crawl_state {
                discord_webhook
                    .lock()
                    .await
                    .send_error(retailer.get_retailer_name(), err)
                    .await;
            }

            discord_webhook
                .lock()
                .await
                .finish_retailer(retailer_name, &results)
                .await;

            if !args.dry_run {
                retailer.emit_metrics();
                db.insert_many_results(results).await;
            }
        }));
    }

    // TODO: fix sync problem
    // this should run after all retailers are initialized
    sleep(Duration::from_secs(2)).await;
    discord_webhook.lock().await.update_main_message().await;

    for handle in handles {
        let _ = handle.await;
    }

    discord_webhook
        .lock()
        .await
        .send_message("Process complete".into())
        .await;

    let _ = PROVIDER.shutdown();
}
