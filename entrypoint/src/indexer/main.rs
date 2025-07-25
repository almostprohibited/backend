use common::result::enums::RetailerName;
use mongodb_connector::connector::MongoDBConnector;
use retailers::{
    pagination_client::PaginationClient,
    retailers::{
        al_flahertys::AlFlahertys, bullseye_north::BullseyeNorth,
        calgary_shooting_centre::CalgaryShootingCentre, canadas_gun_store::CanadasGunStore,
        dante_sports::DanteSports, dominion_outdoors::DominionOutdoors,
        firearms_outlet_canada::FirearmsOutletCanada, g4c_gun_store::G4CGunStore,
        italian_sporting_goods::ItalianSportingGoods, lever_arms::LeverArms,
        rangeview_sports::RangeviewSports, rdsc::Rdsc, reliable_gun::ReliableGun,
        select_shooting_supplies::SelectShootingSupplies, tenda::Tenda,
        the_ammo_source::TheAmmoSource, tillsonburg_gun_shop::Tillsonburg,
        true_north_arms::TrueNorthArms,
    },
    traits::Retailer,
};
use std::sync::Arc;
use tokio::{sync::Mutex, task::JoinHandle};
use tracing::debug;
use utils::{discord::indexer::IndexerWebhook, logger::configure_logger};

#[tokio::main]
async fn main() {
    configure_logger();

    let discord_webhook = Arc::new(Mutex::new(IndexerWebhook::new().await));

    #[cfg(not(debug_assertions))]
    let mut retailers: Vec<Box<dyn Retailer + Send + Sync>> = vec![
        Box::new(AlFlahertys::new()),
        Box::new(BullseyeNorth::new()),
        Box::new(CalgaryShootingCentre::new()),
        Box::new(ReliableGun::new()),
        Box::new(LeverArms::new()),
        Box::new(FirearmsOutletCanada::new()),
        Box::new(CanadasGunStore::new()),
        Box::new(ItalianSportingGoods::new()),
        Box::new(TheAmmoSource::new()),
        Box::new(Rdsc::new()),
        Box::new(G4CGunStore::new()),
        Box::new(Tillsonburg::new()),
        Box::new(DanteSports::new()),
        Box::new(SelectShootingSupplies::new()),
        Box::new(RangeviewSports::new()),
        Box::new(TrueNorthArms::new()),
        Box::new(DominionOutdoors::new()),
    ];

    #[cfg(debug_assertions)]
    let mut retailers: Vec<Box<dyn Retailer + Send + Sync>> = vec![
        Box::new(LeverArms::new()),
        Box::new(SelectShootingSupplies::new()),
    ];

    // tenda requires a special cookie that must be created before
    // any request is allowed through
    #[cfg(not(debug_assertions))]
    match Tenda::new() {
        Ok(tenda) => retailers.push(Box::new(tenda)),
        Err(err) => discord.send_error(RetailerName::Tenda, err).await,
    };

    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    #[cfg(not(debug_assertions))]
    let mongodb = Arc::new(MongoDBConnector::new().await);

    for retailer in retailers {
        #[cfg(not(debug_assertions))]
        let db = mongodb.clone();
        let discord_webhook = discord_webhook.clone();

        handles.push(tokio::spawn(async move {
            let retailer_name = retailer.get_retailer_name().clone();

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
