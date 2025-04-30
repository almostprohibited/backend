use retailers::{reliable_gun::ReliableGun, traits::Retailer};
use tracing::{info, level_filters::LevelFilter};
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

    let reliable_gun = ReliableGun::new().unwrap();
    let result = reliable_gun.get_firearms().await;

    for firearm in result {
        info!("{:?}", firearm);
    }
}
