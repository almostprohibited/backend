use super::constants::{ActionType, AmmunitionType, FirearmClass, FirearmType};

#[derive(Debug, Default)]
pub struct FirearmPrice {
    pub regular_price: u32,
    pub sale_price: Option<u32>,
}

#[derive(Debug, Default)]
pub struct FirearmResult {
    pub name: String,
    pub link: String,
    pub price: FirearmPrice,
    pub description: Option<String>,
    pub action_type: Option<ActionType>,
    pub ammo_type: Option<AmmunitionType>,
    pub firearm_class: Option<FirearmClass>,
    pub firearm_type: Option<FirearmType>,
}

impl FirearmResult {
    pub fn new(name: impl Into<String>, link: impl Into<String>, price: FirearmPrice) -> Self {
        Self {
            name: name.into(),
            link: link.into(),
            price,
            ..Default::default()
        }
    }
}
