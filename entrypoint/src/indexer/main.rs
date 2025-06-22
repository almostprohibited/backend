use mongodb_connector::connector::MongoDBConnector;
use retailers::{
    retailers::{
        al_flahertys::AlFlahertys, bullseye_north::BullseyeNorth, canadas_gun_shop::CanadasGunShop,
        firearmsoutletcanada::FirearmsOutletCanada, lever_arms::LeverArms,
        reliable_gun::ReliableGun,
    },
    traits::Retailer,
};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::debug;
use utils::{discord::indexer::IndexerWebhook, logger::configure_logger};

#[tokio::main]
async fn main() {
    configure_logger();

    let discord = Arc::new(IndexerWebhook::new().await);

    #[cfg(not(debug_assertions))]
    let retailers: Vec<Box<dyn Retailer + Send + Sync>> = vec![
        Box::new(AlFlahertys::new()),
        Box::new(BullseyeNorth::new()),
        // Box::new(CanadasGunShop::new()),
        Box::new(ReliableGun::new()),
        Box::new(LeverArms::new()),
        Box::new(FirearmsOutletCanada::new()),
    ];

    #[cfg(debug_assertions)]
    let retailers: Vec<Box<dyn Retailer + Sync + Send>> = vec![Box::new(CanadasGunShop::new())];

    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    let mongodb = Arc::new(MongoDBConnector::new().await);

    discord
        .send_message("Starting crawling process".into())
        .await;

    for retailer in retailers {
        let db = mongodb.clone();
        let discord = discord.clone();

        handles.push(tokio::spawn(async move {
            let message = format!("{:?} started crawling", retailer.get_retailer_name());
            discord.send_message(message).await;

            let results = retailer.get_crawl_results().await;
            debug!("{:?}", results);

            match results {
                Ok(result) => {
                    db.insert_many_results(result).await;

                    let message = format!("{:?} completed crawling", retailer.get_retailer_name());
                    discord.send_message(message).await;
                }
                Err(err) => discord.send_error(retailer.get_retailer_name(), err).await,
            };
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }

    discord.send_message("Process complete".into()).await;
}
