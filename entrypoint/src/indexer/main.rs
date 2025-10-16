use clap::Parser;
use common::result::enums::RetailerName;
use discord::get_indexer_webhook;
use metrics::_private::PROVIDER;
use mongodb_connector::connector::MongoDBConnector;
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::{debug, info};
use utils::logger::configure_logger;

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

    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    let mongodb = Arc::new(MongoDBConnector::new().await);

    for mut retailer in get_retailers(args.retailers, args.excluded_retailers).await {
        let db = mongodb.clone();

        handles.push(tokio::spawn(async move {
            let retailer_name = retailer.get_retailer_name();

            info!("Executing {retailer_name:?}");

            let crawl_state = retailer.crawl().await;
            let results = retailer.get_results();

            debug!("{:?}", results);

            let mut webhook = get_indexer_webhook().await;

            if let Err(err) = crawl_state {
                webhook.record_retailer_failure(retailer_name, err.to_string());
            }

            webhook.append_retailer_stats(retailer_name, &results);
            webhook.update_main_message().await;

            if !args.dry_run {
                retailer.emit_metrics();
                db.insert_many_results(results).await;
            }
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }

    let mut webhook = get_indexer_webhook().await;

    webhook.finish();
    webhook.update_main_message().await;

    let _ = PROVIDER.shutdown();
}
