use serde::{Deserialize, Serialize};

use crate::result::enums::{ActionType, AmmunitionType, FirearmClass, FirearmType};

#[derive(Deserialize, Serialize, Debug)]
pub enum Metadata {
    Firearm(Firearm),
}

#[derive(Deserialize, Serialize, Debug)]
pub struct Firearm {
    pub action_type: Option<ActionType>,
    pub firearm_type: Option<FirearmType>,
    pub firearm_class: Option<FirearmClass>,
    pub ammo_type: Option<AmmunitionType>,
}
