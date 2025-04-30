use crawler::results::firearm::FirearmResult;

pub trait Retailer {
    fn get_firearms(&self) -> impl Future<Output = Vec<FirearmResult>> + Send;
}
