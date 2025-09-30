use serde_json::Value;
use url::Url;
use wreq::{Client, Method, header::{ORIGIN, REFERER}};

use crate::config::CLAUDE_ENDPOINT;

/// Select a chat-capable organization UUID from the Claude account.
/// Picks the org with the most capabilities that includes "chat".
/// Returns Some(uuid) on success, or None if not found/parse error.
pub async fn select_chat_org_uuid(client: &Client, endpoint: &Url) -> Option<String> {
    let orgs_url = format!(
        "{}/api/organizations",
        endpoint.as_str().trim_end_matches('/')
    );
    let res = client
        .request(Method::GET, orgs_url)
        .header(ORIGIN, CLAUDE_ENDPOINT)
        .header(REFERER, format!("{}/new", CLAUDE_ENDPOINT))
        .send()
        .await
        .ok()?;
    let val: Value = res.json().await.ok()?;
    val.as_array()
        .and_then(|a| {
            a.iter()
                .filter(|v| {
                    v.get("capabilities")
                        .and_then(|c| c.as_array())
                        .map(|c| c.iter().any(|x| x.as_str() == Some("chat")))
                        .unwrap_or(false)
                })
                .max_by_key(|v| {
                    v.get("capabilities")
                        .and_then(|c| c.as_array())
                        .map(|c| c.len())
                        .unwrap_or_default()
                })
                .and_then(|v| v.get("uuid").and_then(|u| u.as_str()))
                .map(|s| s.to_string())
        })
        .or_else(|| {
            val.get(0)
                .and_then(|v| v.get("uuid").and_then(|u| u.as_str()))
                .map(|s| s.to_string())
        })
}

/// Select a chat-capable organization UUID from an organizations JSON array response.
/// Mirrors the same preference logic as `select_chat_org_uuid`.
pub fn select_chat_org_uuid_from_value(val: &serde_json::Value) -> Option<String> {
    val.as_array()
        .and_then(|a| {
            a.iter()
                .filter(|v| {
                    v.get("capabilities")
                        .and_then(|c| c.as_array())
                        .map(|c| c.iter().any(|x| x.as_str() == Some("chat")))
                        .unwrap_or(false)
                })
                .max_by_key(|v| {
                    v.get("capabilities")
                        .and_then(|c| c.as_array())
                        .map(|c| c.len())
                        .unwrap_or_default()
                })
                .and_then(|v| v.get("uuid").and_then(|u| u.as_str()))
                .map(|s| s.to_string())
        })
        .or_else(|| {
            val.get(0)
                .and_then(|v| v.get("uuid").and_then(|u| u.as_str()))
                .map(|s| s.to_string())
        })
}
