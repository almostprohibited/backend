use serde::Deserialize;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Provider {
    Discord,
    // Google,
    // Facebook,
}
