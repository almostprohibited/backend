use mongodb_connector::connector::MongoDBConnector;
use retailers::{
    retailers::{
        al_flahertys::AlFlahertys, bullseye_north::BullseyeNorth, canadas_gun_shop::CanadasGunShop,
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

    let retailers: Vec<Box<dyn Retailer + Send + Sync>> = vec![
        // disabled, they don't seem to be able to implement Cloudflare properly
        // and instead have a jank recaptcha that doesn't work half the time
        // note: unimplemented
        // Box::new(ArmsEast::new()),
        Box::new(AlFlahertys::new()),
        Box::new(BullseyeNorth::new()),
        Box::new(CanadasGunShop::new()),
        // Box::new(CanadasGunStore::new()),
        Box::new(ReliableGun::new()),
        // disable ISG, they appear to have ArsenalForce specified in https://www.italiansportinggoods.com/robots.txt
        // TODO: talk to them instead of just crawling anyways
        // Box::new(ItalianSportingGoods::new()),
        Box::new(LeverArms::new()),
        // disable ISS, they appear to have ArsenalForce specified in https://internationalshootingsupplies.com/robots.txt
        // TODO: talk to them instead of just crawling anyways
        // note: unimplemented
        // Box::new(InternationalShootingSupplies::new()),
    ];

    // let retailers: Vec<Box<dyn Retailer + Sync + Send>> = vec![Box::new(ReliableGun::new())];

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
