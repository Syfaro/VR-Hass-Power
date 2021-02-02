//! Home Assistant API access.

use serde::{Deserialize, Serialize};

use crate::config::HomeAssistantConfig;

/// An on or off value for Home Assistant.
#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum APIState {
    On,
    Off,
}

/// The current on or off value for an entity's state.
#[derive(Debug, Deserialize)]
struct APIStateResponse {
    /// The state of the entity.
    state: APIState,
}

/// The body needed to call the service to turn a device on or off.
#[derive(Debug, Serialize)]
struct APIServiceCall {
    /// The ID of the entity to control.
    entity_id: String,
}

/// Check the provided credentials by ensuring a valid API response is received.
pub fn check_credentials(config: &HomeAssistantConfig) -> bool {
    ureq::get(&format!("{}/api/", config.url))
        .set("Authorization", &format!("Bearer {}", config.api_key))
        .call()
        .is_ok()
}

/// Get the state of a Home Assistant entity.
pub fn get_entity_state(
    config: &HomeAssistantConfig,
) -> Result<APIState, Box<dyn std::error::Error>> {
    let resp: APIStateResponse = ureq::get(&format!("{}/api/states/{}", config.url, config.entity))
        .set("Authorization", &format!("Bearer {}", config.api_key))
        .call()?
        .into_json()?;

    Ok(resp.state)
}

/// Set the state of a Home Assistant entity.
pub fn set_entity_state(
    config: &HomeAssistantConfig,
    state: APIState,
) -> Result<(), Box<dyn std::error::Error>> {
    let service = match state {
        APIState::On => "turn_on",
        APIState::Off => "turn_off",
    };

    ureq::post(&format!(
        "{}/api/services/{}/{}",
        config.url, config.service, service
    ))
    .set("Authorization", &format!("Bearer {}", config.api_key))
    .send_json(
        serde_json::to_value(APIServiceCall {
            entity_id: config.entity.to_string(),
        })
        .unwrap(),
    )?;

    Ok(())
}
