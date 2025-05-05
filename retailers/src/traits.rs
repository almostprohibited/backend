use async_trait::async_trait;
use crawler::results::{
    constants::{ActionType, AmmunitionType, FirearmClass, FirearmType},
    firearm::FirearmResult,
};

#[async_trait]
pub trait Retailer {
    async fn get_firearms(&self) -> Vec<FirearmResult>;
}

pub(crate) struct SearchParams<'a> {
    pub(crate) lookup: &'a str,
    pub(crate) action_type: ActionType,
    pub(crate) ammo_type: AmmunitionType,
    pub(crate) firearm_class: FirearmClass,
    pub(crate) firearm_type: FirearmType,
}
