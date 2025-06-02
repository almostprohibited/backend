use mongodb_connector::connector::MongoDBConnector;
use retailers::{
    retailers::{
        al_flahertys::AlFlahertys, bullseye_north::BullseyeNorth, canadas_gun_shop::CanadasGunShop,
        canadas_gun_store::CanadasGunStore, lever_arms::LeverArms, reliable_gun::ReliableGun,
    },
    traits::Retailer,
};
use std::sync::Arc;
use tokio::task::JoinHandle;
use tracing::debug;
use utils::{discord::Discord, logger::configure_logger};

#[tokio::main]
async fn main() {
    configure_logger();

    let discord = Arc::new(Discord::new().await);

    // let retailers: Vec<Box<dyn Retailer + Send + Sync>> = vec![
    //     // disabled, they don't seem to be able to implement Cloudflare properly
    //     // and instead have a jank recaptcha that doesn't work half the time
    //     // note: unimplemented
    //     // Box::new(ArmsEast::new()),
    //     Box::new(AlFlahertys::new()),
    //     Box::new(BullseyeNorth::new()),
    //     Box::new(CanadasGunShop::new()),
    //     Box::new(CanadasGunStore::new()),
    //     Box::new(ReliableGun::new()),
    //     // disable ISG, they appear to have ArsenalForce specified in https://www.italiansportinggoods.com/robots.txt
    //     // TODO: talk to them instead of just crawling anyways
    //     //Box::new(ItalianSportingGoods::new()),
    //     Box::new(LeverArms::new()),
    //     // disable ISS, they appear to have ArsenalForce specified in https://internationalshootingsupplies.com/robots.txt
    //     // TODO: talk to them instead of just crawling anyways
    //     // note: unimplemented
    //     //Box::new(InternationalShootingSupplies::new()),
    // ];

    let retailers: Vec<Box<dyn Retailer + Sync + Send>> = vec![Box::new(AlFlahertys::new())];

    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    let mongodb = Arc::new(MongoDBConnector::new().await);

    for retailer in retailers {
        let db = mongodb.clone();
        let discord = discord.clone();

        handles.push(tokio::spawn(async move {
            let result = retailer.get_firearms().await;
            debug!("{:?}", result);

            match result {
                Ok(firearms) => db.insert_many_firearms(firearms).await,
                Err(err) => discord.send_error(err).await,
            };
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }
}
