use clap::Parser;
use common::result::enums::RetailerName;
use mongodb_connector::connector::MongoDBConnector;
use retailers::pagination_client::PaginationClient;
use std::{sync::Arc, time::Duration};
use tokio::{sync::Mutex, task::JoinHandle, time::sleep};
use tracing::{debug, info};
use utils::{discord::indexer::IndexerWebhook, logger::configure_logger};

use crate::retailer_helpers::get_retailers;

mod retailer_helpers;

#[derive(Parser)]
#[command(version)]
struct Arguments {
    #[arg(short, long, value_delimiter = ' ', num_args = 0..)]
    retailers: Vec<RetailerName>,
}

#[tokio::main]
async fn main() {
    let args = Arguments::parse();

    configure_logger();

    let discord_webhook = Arc::new(Mutex::new(IndexerWebhook::new().await));

    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    #[cfg(not(debug_assertions))]
    let mongodb = Arc::new(MongoDBConnector::new().await);

    for retailer in get_retailers(args.retailers) {
        #[cfg(not(debug_assertions))]
        let db = mongodb.clone();
        let discord_webhook = discord_webhook.clone();

        handles.push(tokio::spawn(async move {
            let retailer_name = retailer.get_retailer_name().clone();
            info!("Registering {retailer_name:?}");

            discord_webhook
                .lock()
                .await
                .register_retailer(retailer_name);

            let mut pagination_client = PaginationClient::new(retailer);

            let crawl_state = pagination_client.crawl().await;
            let results = pagination_client.get_results();

            debug!("{:?}", results);

            if let Err(err) = crawl_state {
                discord_webhook
                    .lock()
                    .await
                    .send_error(pagination_client.get_retailer_name(), err)
                    .await;
            }

            discord_webhook
                .lock()
                .await
                .finish_retailer(retailer_name, &results, pagination_client.total_bytes_tx)
                .await;

            #[cfg(not(debug_assertions))]
            db.insert_many_results(results).await;
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
}
