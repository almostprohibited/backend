use serde::{Deserialize, Serialize};

use crate::result::enums::{ActionType, AmmunitionType, FirearmClass, FirearmType};

#[derive(Deserialize, Serialize, Debug)]
pub enum Metadata {
    Firearm(Firearm),
    Ammunition(Ammunition),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Firearm {
    pub action_type: Option<ActionType>,
    pub firearm_type: Option<FirearmType>,
    pub firearm_class: Option<FirearmClass>,
    pub ammo_type: Option<AmmunitionType>,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Ammunition {
    pub round_count: Option<u64>,
    // grains is defined as String to account for
    // shotgun shell length, I know it's not "grains"
    // at this point
    pub grains: Option<String>,
    pub brand: Option<String>,
    pub caliber: Option<String>,
    pub model: Option<String>,
}

impl Ammunition {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_round_count(mut self, count: u64) -> Self {
        self.round_count = Some(count);
        self
    }
}
