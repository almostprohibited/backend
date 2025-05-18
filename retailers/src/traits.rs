use async_trait::async_trait;

use crate::results::{
    constants::{ActionType, AmmunitionType, FirearmClass, FirearmType},
    firearm::FirearmResult,
};

#[async_trait]
pub trait Retailer {
    async fn get_firearms(&self) -> Vec<FirearmResult>;
}

pub(crate) struct SearchParams<'a> {
    pub(crate) lookup: &'a str,
    pub(crate) action_type: Option<ActionType>,
    pub(crate) ammo_type: Option<AmmunitionType>,
    pub(crate) firearm_class: Option<FirearmClass>,
    pub(crate) firearm_type: Option<FirearmType>,
}
