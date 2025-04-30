use super::constants::{ActionType, AmmunitionType, FirearmClass, FirearmType};

#[derive(Debug, Default)]
pub struct FirearmResult {
    pub name: String,
    pub link: String,
    pub cost_cents: u64,

    pub description: Option<String>,
    pub action_type: Option<ActionType>,
    pub ammo_type: Option<AmmunitionType>,
    pub firearm_class: Option<FirearmClass>,
    pub firearm_type: Option<FirearmType>,
}

impl FirearmResult {
    pub fn new(name: impl Into<String>, link: impl Into<String>, cost_cents: u64) -> Self {
        Self {
            name: name.into(),
            link: link.into(),
            cost_cents,
            ..Default::default()
        }
    }
}
