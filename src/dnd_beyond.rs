use std::{env, time::Duration};

use reqwest::blocking::Client;
use reqwest::header::{
    ACCEPT, AUTHORIZATION, COOKIE, HeaderMap, HeaderName, HeaderValue, REFERER, USER_AGENT,
};
use serde::Deserialize;

use crate::models::creature::Creature;

const CHARACTER_URL_PREFIX: &str = "https://character-service.dndbeyond.com/character/v5/character";
const COOKIE_ENV_VAR: &str = "DND_BEYOND_COOKIE";
const AUTHORIZATION_ENV_VAR: &str = "DND_BEYOND_AUTHORIZATION";
const BEARER_TOKEN_ENV_VAR: &str = "DND_BEYOND_BEARER_TOKEN";
const ORIGIN: HeaderName = HeaderName::from_static("origin");

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CharacterId(pub u64);

pub fn fetch_character(id: CharacterId) -> color_eyre::Result<Creature> {
    // This function is called from synchronous UI/storage code that runs inside Tokio.
    // reqwest::blocking creates its own runtime internally, and dropping that runtime
    // from Tokio's async context can panic. Keep the blocking client isolated on a
    // plain OS thread until the storage path is made fully async.
    std::thread::spawn(move || fetch_character_on_blocking_thread(id))
        .join()
        .map_err(|_| color_eyre::eyre::eyre!("D&D Beyond fetch thread panicked"))?
}

fn fetch_character_on_blocking_thread(id: CharacterId) -> color_eyre::Result<Creature> {
    let url = format!("{CHARACTER_URL_PREFIX}/{}", id.0);
    let response = client()?.get(url).send()?;
    let status = response.status();
    let body = response.text()?;

    if !status.is_success() {
        color_eyre::eyre::bail!(
            "D&D Beyond returned HTTP {status} for character {}: {}",
            id.0,
            body.trim()
        );
    }

    let envelope: CharacterResponseEnvelope = serde_json::from_str(&body)?;

    if !envelope.success {
        color_eyre::eyre::bail!(
            "D&D Beyond rejected character {}: {}",
            id.0,
            envelope
                .message
                .unwrap_or_else(|| "unknown error".to_string())
        );
    }

    let data = envelope.data.ok_or_else(|| {
        color_eyre::eyre::eyre!(
            "D&D Beyond returned success without character data for {}",
            id.0
        )
    })?;
    let data: CharacterData = serde_json::from_value(data)?;
    Ok(data.into_creature())
}

fn client() -> color_eyre::Result<Client> {
    let mut headers = HeaderMap::new();
    headers.insert(
        USER_AGENT,
        HeaderValue::from_static(
            "Mozilla/5.0 (X11; Linux x86_64; rv:150.0) Gecko/20100101 Firefox/150.0",
        ),
    );
    headers.insert(
        ACCEPT,
        HeaderValue::from_static("application/json, text/plain, */*"),
    );
    headers.insert(
        ORIGIN,
        HeaderValue::from_static("https://www.dndbeyond.com"),
    );
    headers.insert(
        REFERER,
        HeaderValue::from_static("https://www.dndbeyond.com/"),
    );

    if let Ok(authorization) = env::var(AUTHORIZATION_ENV_VAR) {
        if let Some(authorization) = authorization_header_value(&authorization)? {
            headers.insert(AUTHORIZATION, authorization);
        }
    } else if let Ok(token) = env::var(BEARER_TOKEN_ENV_VAR) {
        if let Some(authorization) = authorization_header_value(&token)? {
            headers.insert(AUTHORIZATION, authorization);
        }
    }

    if let Ok(cookie) = env::var(COOKIE_ENV_VAR) {
        if !cookie.trim().is_empty() {
            headers.insert(COOKIE, HeaderValue::from_str(cookie.trim())?);
        }
    }

    Ok(Client::builder()
        .default_headers(headers)
        .timeout(Duration::from_secs(15))
        .build()?)
}

fn authorization_header_value(value: &str) -> color_eyre::Result<Option<HeaderValue>> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(None);
    }

    if value.starts_with("Bearer ") {
        Ok(Some(HeaderValue::from_str(value)?))
    } else {
        Ok(Some(HeaderValue::from_str(&format!("Bearer {value}"))?))
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CharacterResponseEnvelope {
    success: bool,
    message: Option<String>,
    data: Option<serde_json::Value>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CharacterData {
    name: String,
    base_hit_points: i32,
    bonus_hit_points: Option<i32>,
    override_hit_points: Option<i32>,
    removed_hit_points: i32,
    #[serde(default)]
    armor_class: Option<i32>,
    #[serde(default)]
    race: Option<Race>,
    #[serde(default)]
    classes: Vec<Class>,
}

impl CharacterData {
    fn into_creature(self) -> Creature {
        let max_health = self
            .override_hit_points
            .unwrap_or_else(|| self.base_hit_points + self.bonus_hit_points.unwrap_or_default());
        let current_health = max_health - self.removed_hit_points;
        let description = self.description();
        let mut creature = Creature::new(self.name, None, self.armor_class, max_health);
        creature.set_health(current_health);
        creature.set_description(description);
        creature
    }

    fn description(&self) -> String {
        let ancestry = self.race.as_ref().map(|race| race.full_name.as_str());
        let classes = self
            .classes
            .iter()
            .map(Class::description)
            .collect::<Vec<_>>();

        match (ancestry, classes.is_empty()) {
            (Some(ancestry), false) => format!("{} {}", ancestry, classes.join(" / ")),
            (Some(ancestry), true) => ancestry.to_string(),
            (None, false) => classes.join(" / "),
            (None, true) => "D&D Beyond character".to_string(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Race {
    full_name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Class {
    level: i32,
    definition: ClassDefinition,
}

impl Class {
    fn description(&self) -> String {
        format!("{} {}", self.definition.name, self.level)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ClassDefinition {
    name: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dnd_beyond_response_uses_reported_armor_class_only() {
        let data: CharacterData = serde_json::from_value(serde_json::json!({
            "name": "Example Hero",
            "baseHitPoints": 42,
            "bonusHitPoints": 3,
            "overrideHitPoints": null,
            "removedHitPoints": 7,
            "race": { "fullName": "Human" },
            "classes": [{ "level": 6, "definition": { "name": "Fighter" } }]
        }))
        .unwrap();

        let creature = data.into_creature();
        assert_eq!(creature.name, "Example Hero");
        assert_eq!(creature.get_health(), 38);
        assert_eq!(creature.get_max_health(), 45);
        assert_eq!(creature.ac, None);
        assert_eq!(creature.description, "Human Fighter 6");
    }

    #[test]
    fn dnd_beyond_response_preserves_reported_armor_class_when_present() {
        let data: CharacterData = serde_json::from_value(serde_json::json!({
            "name": "Example Hero",
            "baseHitPoints": 42,
            "bonusHitPoints": null,
            "overrideHitPoints": 50,
            "removedHitPoints": 0,
            "armorClass": 17
        }))
        .unwrap();

        let creature = data.into_creature();
        assert_eq!(creature.get_max_health(), 50);
        assert_eq!(creature.ac, Some(17));
    }

    #[test]
    fn unsuccessful_response_reports_api_message_without_deserializing_character_data() {
        let envelope: CharacterResponseEnvelope = serde_json::from_value(serde_json::json!({
            "success": false,
            "message": "An unexpected error has occurred",
            "data": {
                "serverMessage": "Unauthorized Access Attempt.",
                "errorCode": "7c0fc77"
            }
        }))
        .unwrap();

        assert!(!envelope.success);
        assert_eq!(
            envelope.message.as_deref(),
            Some("An unexpected error has occurred")
        );
    }
}
