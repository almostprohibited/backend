use mongodb_connector::connector::MongoDBConnector;
use retailers::{
    retailers::{
        al_flahertys::AlFlahertys, bullseye_north::BullseyeNorth,
        calgary_shooting_centre::CalgaryShootingCentre, canadas_gun_store::CanadasGunStore,
        firearmsoutletcanada::FirearmsOutletCanada, italian_sporting_goods::ItalianSportingGoods,
        lever_arms::LeverArms, reliable_gun::ReliableGun,
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
        Box::new(CalgaryShootingCentre::new()),
        Box::new(ReliableGun::new()),
        Box::new(LeverArms::new()),
        Box::new(FirearmsOutletCanada::new()),
        Box::new(CanadasGunStore::new()),
        Box::new(ItalianSportingGoods::new()),
    ];

    #[cfg(debug_assertions)]
    let retailers: Vec<Box<dyn Retailer + Sync + Send>> =
        vec![Box::new(ItalianSportingGoods::new())];

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
                    let message = format!(
                        "{:?} completed crawling ({} items)",
                        retailer.get_retailer_name(),
                        result.len()
                    );
                    discord.send_message(message).await;

                    #[cfg(not(debug_assertions))]
                    db.insert_many_results(result).await;
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
