use mongodb_connector::connector::MongoDBConnector;
use retailers::{
    pagination_client::PaginationClient,
    retailers::{
        al_flahertys::AlFlahertys, bullseye_north::BullseyeNorth,
        calgary_shooting_centre::CalgaryShootingCentre, canadas_gun_store::CanadasGunStore,
        firearmsoutletcanada::FirearmsOutletCanada, italian_sporting_goods::ItalianSportingGoods,
        lever_arms::LeverArms, reliable_gun::ReliableGun, theammosource::TheAmmoSource,
    },
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
    let retailers: Vec<PaginationClient> = vec![
        PaginationClient::new(Box::new(AlFlahertys::new())),
        PaginationClient::new(Box::new(BullseyeNorth::new())),
        PaginationClient::new(Box::new(CalgaryShootingCentre::new())),
        PaginationClient::new(Box::new(ReliableGun::new())),
        PaginationClient::new(Box::new(LeverArms::new())),
        PaginationClient::new(Box::new(FirearmsOutletCanada::new())),
        PaginationClient::new(Box::new(CanadasGunStore::new())),
        PaginationClient::new(Box::new(ItalianSportingGoods::new())),
        PaginationClient::new(Box::new(TheAmmoSource::new())),
    ];

    #[cfg(debug_assertions)]
    let retailers: Vec<PaginationClient> =
        vec![PaginationClient::new(Box::new(TheAmmoSource::new()))];

    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    let mongodb = Arc::new(MongoDBConnector::new().await);

    discord
        .send_message("Starting crawling process".into())
        .await;

    for mut retailer in retailers {
        let db = mongodb.clone();
        let discord = discord.clone();

        handles.push(tokio::spawn(async move {
            let start_message = format!("{:?} started crawling", retailer.get_retailer_name());
            discord.send_message(start_message).await;

            let crawl_state = retailer.crawl().await;
            let results = retailer.get_results();

            debug!("{:?}", results);

            if let Err(err) = crawl_state {
                discord.send_error(retailer.get_retailer_name(), err).await;
            }

            let finished_message = format!(
                "{:?} completed crawling ({} items)",
                retailer.get_retailer_name(),
                results.len()
            );
            discord.send_message(finished_message).await;

            #[cfg(not(debug_assertions))]
            db.insert_many_results(results).await;
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }

    discord.send_message("Process complete".into()).await;
}
