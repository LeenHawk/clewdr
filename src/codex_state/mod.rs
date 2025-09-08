use std::sync::LazyLock;

use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use http::header::{ACCEPT, CONTENT_TYPE};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use snafu::ResultExt;
use tracing::warn;
use uuid::Uuid;
use wreq::{Client, ClientBuilder, Method};

use crate::{
    config::CLEWDR_CONFIG,
    error::{ClewdrError, WreqSnafu},
    types::claude::{ContentBlock, Message, MessageContent, Role},
};

pub static SUPER_CLIENT: LazyLock<Client> = LazyLock::new(Client::new);

#[derive(Clone, Default)]
pub struct CodexState {
    pub client: Client,
}

impl CodexState {
    pub fn new() -> Self {
        let mut builder = ClientBuilder::new().cookie_store(false);
        if let Some(p) = &CLEWDR_CONFIG.load().wreq_proxy {
            builder = builder.proxy(p.to_owned());
        }
        let client = builder.build().unwrap_or_else(|_| SUPER_CLIENT.to_owned());
        Self { client }
    }

    pub fn normalize_model_name(&self, name: Option<&str>) -> String {
        let Some(name) = name.map(|s| s.trim()).filter(|s| !s.is_empty()) else {
            return "gpt-5".to_string();
        };
        let mut base = name.split(':').next().unwrap_or(name).trim().to_string();
        for sep in ['-', '_'] {
            let lowered = base.to_lowercase();
            for effort in ["minimal", "low", "medium", "high"] {
                let suffix = format!("{}{}", sep, effort);
                if lowered.ends_with(&suffix) {
                    let n = base.len() - suffix.len();
                    base.truncate(n);
                    break;
                }
            }
        }
        match base.as_str() {
            "gpt5" | "gpt-5-latest" | "gpt-5" => "gpt-5".to_string(),
            "codex" | "codex-mini" | "codex-mini-latest" => "codex-mini-latest".to_string(),
            _ => base,
        }
    }

    /// Convert OpenAI messages to ChatGPT Responses input items
    pub fn convert_messages_to_responses_input(&self, messages: &[Message]) -> Vec<Value> {
        let mut out: Vec<Value> = vec![];
        for msg in messages.iter() {
            match msg.role {
                Role::System => {
                    // Move to instructions outside; ignore here
                }
                Role::Assistant => {
                    // function call support for assistant.tool_calls lives in OAI-compatible input
                    if let MessageContent::Blocks { content } = &msg.content {
                        // Also capture assistant emitted text
                        let mut items: Vec<Value> = vec![];
                        for blk in content.iter() {
                            match blk {
                                ContentBlock::Text { text } => {
                                    if !text.is_empty() {
                                        items.push(json!({"type": "output_text", "text": text}));
                                    }
                                }
                                _ => {}
                            }
                        }
                        if !items.is_empty() {
                            out.push(json!({
                                "type": "message",
                                "role": "assistant",
                                "content": items,
                            }));
                        }
                    } else if let MessageContent::Text { content } = &msg.content {
                        if !content.is_empty() {
                            out.push(json!({
                                "type": "message",
                                "role": "assistant",
                                "content": [{"type": "output_text", "text": content}],
                            }));
                        }
                    }
                }
                _ => {
                    // user/tool
                    if let MessageContent::Blocks { content } = &msg.content {
                        // tool result from tools messages
                        // Also handle image_url/text
                        // tool role: ContentBlock::ToolResult? But in OAI tool role arrives differently; we support 'tool' role via ToolResult mapping below.
                        if matches!(msg.role, Role::Assistant) {
                            // already handled above
                        }
                        // Map content items
                        let mut items: Vec<Value> = vec![];
                        for blk in content.iter() {
                            match blk {
                                ContentBlock::Text { text } => {
                                    if !text.is_empty() {
                                        let kind = if matches!(msg.role, Role::Assistant) {
                                            "output_text"
                                        } else {
                                            "input_text"
                                        };
                                        items.push(json!({"type": kind, "text": text}));
                                    }
                                }
                                ContentBlock::ImageUrl { image_url } => {
                                    let url = normalize_data_url(&image_url.url);
                                    if !url.is_empty() {
                                        items
                                            .push(json!({"type": "input_image", "image_url": url}));
                                    }
                                }
                                ContentBlock::ToolResult {
                                    tool_use_id,
                                    content,
                                } => {
                                    // Map Claude style tool_result to function_call_output
                                    items.push(json!({
                                        "type": "function_call_output",
                                        "call_id": tool_use_id,
                                        "output": content,
                                    }));
                                }
                                _ => {}
                            }
                        }
                        if !items.is_empty() {
                            let role = if matches!(msg.role, Role::Assistant) {
                                "assistant"
                            } else {
                                "user"
                            };
                            out.push(json!({"type": "message", "role": role, "content": items}));
                        }
                    } else if let MessageContent::Text { content } = &msg.content {
                        if !content.is_empty() {
                            let kind = if matches!(msg.role, Role::Assistant) {
                                "output_text"
                            } else {
                                "input_text"
                            };
                            let role = if matches!(msg.role, Role::Assistant) {
                                "assistant"
                            } else {
                                "user"
                            };
                            out.push(json!({
                                "type": "message",
                                "role": role,
                                "content": [{"type": kind, "text": content}],
                            }));
                        }
                    }
                }
            }
        }
        out
    }

    pub async fn start_upstream(
        &self,
        model: &str,
        instructions: Option<String>,
        input_items: Vec<Value>,
        tools: Vec<Value>,
        tool_choice: Value,
        parallel_tool_calls: bool,
        reasoning: Option<Value>,
        session_id: Option<String>,
    ) -> Result<wreq::Response, ClewdrError> {
        let access_token = CLEWDR_CONFIG
            .load()
            .codex
            .tokens
            .access_token
            .clone()
            .ok_or(ClewdrError::BadRequest {
                msg: "Codex not authenticated. Use /api/codex/oauth/start".into(),
            })?;
        let account_id = CLEWDR_CONFIG.load().codex.tokens.account_id.clone().ok_or(
            ClewdrError::BadRequest {
                msg: "Codex missing account_id".into(),
            },
        )?;

        let mut include: Vec<&'static str> = vec![];
        if reasoning.is_some() {
            include.push("reasoning.encrypted_content");
        }

        let sid =
            session_id.unwrap_or_else(|| ensure_session_id(instructions.as_deref(), &input_items));
        let mut payload = json!({
            "model": model,
            "instructions": instructions,
            "input": input_items,
            "tools": tools,
            "tool_choice": tool_choice,
            "parallel_tool_calls": parallel_tool_calls,
            "store": false,
            "stream": true,
            "prompt_cache_key": sid,
        });
        if !include.is_empty() {
            payload["include"] = json!(include);
        }
        if let Some(r) = reasoning {
            payload["reasoning"] = r;
        }

        let url = "https://chatgpt.com/backend-api/codex/responses";
        let req = self
            .client
            .request(Method::POST, url)
            .header(ACCEPT, "text/event-stream")
            .header(CONTENT_TYPE, "application/json")
            .header("Authorization", format!("Bearer {}", access_token))
            .header("chatgpt-account-id", account_id)
            .header("OpenAI-Beta", "responses=experimental")
            .header("session_id", sid)
            .json(&payload);
        Ok(req.send().await.context(WreqSnafu {
            msg: "Codex upstream request failed",
        })?)
    }
}

/// Generate a deterministic session id from instructions + first user message
pub fn ensure_session_id(instructions: Option<&str>, input_items: &[Value]) -> String {
    let mut prefix = String::new();
    if let Some(ins) = instructions {
        if !ins.trim().is_empty() {
            prefix.push_str(ins.trim());
        }
    }
    if let Some(first_user) = canonicalize_first_user_message(input_items) {
        prefix.push_str(&first_user);
    }
    if prefix.is_empty() {
        return Uuid::new_v4().to_string();
    }
    let mut hasher = Sha256::new();
    hasher.update(prefix.as_bytes());
    let digest = hasher.finalize();
    hex::encode(digest)
}

fn canonicalize_first_user_message(input_items: &[Value]) -> Option<String> {
    for item in input_items {
        let t = item.get("type")?.as_str()?;
        if t != "message" {
            continue;
        }
        let role = item.get("role")?.as_str()?;
        if role != "user" {
            continue;
        }
        let content = item.get("content")?.as_array()?;
        let mut out_parts: Vec<String> = vec![];
        for part in content {
            let ptype = part.get("type").and_then(|v| v.as_str());
            match ptype {
                Some("input_text") => {
                    if let Some(text) = part.get("text").and_then(|v| v.as_str()) {
                        out_parts.push(text.to_string());
                    }
                }
                Some("input_image") => {
                    if let Some(url) = part.get("image_url").and_then(|v| v.as_str()) {
                        out_parts.push(format!("<img:{}>", url));
                    }
                }
                _ => {}
            }
        }
        if !out_parts.is_empty() {
            return Some(out_parts.join("|"));
        }
    }
    None
}

fn normalize_data_url(url: &str) -> String {
    if !url.starts_with("data:image/") {
        return url.to_string();
    }
    if let Some((header, data)) = url.split_once(',') {
        let d = data.trim().replace('\n', "").replace('\r', "");
        let d = d.replace('-', "+").replace('_', "/");
        let pad = (4 - d.len() % 4) % 4;
        let mut dpad = d;
        if pad > 0 {
            dpad.push_str(&"=".repeat(pad));
        }
        if URL_SAFE_NO_PAD.decode(dpad.as_bytes()).is_err()
            && base64::engine::general_purpose::STANDARD
                .decode(dpad.as_bytes())
                .is_err()
        {
            warn!("Invalid base64 image data");
            return url.to_string();
        }
        format!("{},{}", header, dpad)
    } else {
        url.to_string()
    }
}
