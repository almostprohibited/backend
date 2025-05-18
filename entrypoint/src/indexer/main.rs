use std::sync::Arc;

use mongodb_connector::connector::MongoDBConnector;
use retailers::{
    retailers::{
        italian_sporting_goods::ItalianSportingGoods, lever_arms::LeverArms,
        reliable_gun::ReliableGun,
    },
    traits::Retailer,
};
use tokio::task::JoinHandle;
use tracing::{debug, info, level_filters::LevelFilter};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

#[tokio::main]
async fn main() {
    let env_log = EnvFilter::builder()
        .with_default_directive(LevelFilter::INFO.into())
        .from_env()
        .expect("Failed to create tracing filter");

    let subscriber = FmtSubscriber::builder()
        .pretty()
        .compact()
        .with_file(false)
        .with_env_filter(env_log);

    tracing::subscriber::set_global_default(subscriber.finish())
        .expect("Failed to create log subscription");

    let retailers: Vec<Box<dyn Retailer + Send>> = vec![
        Box::new(ReliableGun::new()),
        // disable ISG, they appear to have ArsenalForce specified in https://www.italiansportinggoods.com/robots.txt
        //Box::new(ItalianSportingGoods::new()),
        Box::new(LeverArms::new()),
        // disable ISS, they appear to have ArsenalForce specified in https://internationalshootingsupplies.com/robots.txt
        // note: crawler not defined yet
        //Box::new(InternationalShootingSupplies::new()),
    ];

    let mut handles: Vec<JoinHandle<()>> = Vec::new();

    let mongodb = Arc::new(MongoDBConnector::new().await);

    for retailer in retailers {
        let db = mongodb.clone();

        handles.push(tokio::spawn(async move {
            let firearms = retailer.get_firearms().await;
            debug!("{:?}", firearms);
            db.insert_many_firearms(firearms).await;
        }));
    }

    for handle in handles {
        let _ = handle.await;
    }
}
