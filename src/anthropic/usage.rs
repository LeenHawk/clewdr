use serde_json::Value;
use wreq::{Client, Method};

use crate::config::CLAUDE_CONSOLE_ENDPOINT;

/// Fetch usage JSON from Anthropic Console API for a given organization.
/// Returns None on network/parse errors.
pub async fn fetch_console_usage(client: &Client, org_uuid: &str) -> Option<Value> {
    let usage_url = format!(
        "{}/api/organizations/{}/usage",
        CLAUDE_CONSOLE_ENDPOINT, org_uuid
    );
    let res = client.request(Method::GET, usage_url).send().await.ok()?;
    res.json().await.ok()
}

/// Parse percentage metrics and ISO reset timestamps from usage payload.
/// Returns (five_hour%, five_hour.resets_at, seven_day%, seven_day.resets_at, seven_day_opus%, seven_day_opus.resets_at)
pub fn parse_usage_percents(usage: &Value) -> (u32, Option<String>, u32, Option<String>, u32, Option<String>) {
    let five = usage
        .get("five_hour")
        .and_then(|o| o.get("utilization"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let five_reset = usage
        .get("five_hour")
        .and_then(|o| o.get("resets_at"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let seven = usage
        .get("seven_day")
        .and_then(|o| o.get("utilization"))
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as u32;
    let seven_reset = usage
        .get("seven_day")
        .and_then(|o| o.get("resets_at"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let seven_opus = usage
        .get("seven_day_opus")
        .and_then(|o| o.get("utilization"))
        .and_then(|v| v.as_u64())
        .map(|v| v as u32)
        .unwrap_or(0);
    let opus_reset = usage
        .get("seven_day_opus")
        .and_then(|o| o.get("resets_at"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    (five, five_reset, seven, seven_reset, seven_opus, opus_reset)
}

/// Parse reset timestamps (RFC3339 -> epoch seconds) for session/weekly/weekly_opus windows.
pub fn parse_reset_timestamps(usage: &Value) -> (Option<i64>, Option<i64>, Option<i64>) {
    let parse_reset = |obj_key: &str| -> Option<i64> {
        usage
            .get(obj_key)
            .and_then(|o| o.get("resets_at"))
            .and_then(|v| v.as_str())
            .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
            .map(|dt| dt.timestamp())
    };
    (
        parse_reset("five_hour"),
        parse_reset("seven_day"),
        parse_reset("seven_day_opus"),
    )
}

