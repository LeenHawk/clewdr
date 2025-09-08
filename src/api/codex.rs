use std::time::SystemTime;

use axum::{
    Json,
    extract::State,
    response::{IntoResponse, Response, Sse},
};
use eventsource_stream::Eventsource;
use futures::TryStreamExt;
use serde_json::{Value, json};

use crate::{
    codex_state::CodexState,
    error::ClewdrError,
    types::claude::{Message, Role},
    types::oai::CreateMessageParams as OaiCreateMessageParams,
};

#[derive(Clone, Default)]
pub struct CodexApiState {
    pub state: CodexState,
}

fn unix_time() -> i64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or_else(|_| 0)
}

fn system_instructions(messages: &[Message]) -> Option<String> {
    let mut parts: Vec<String> = vec![];
    for m in messages.iter() {
        if m.role != Role::System {
            continue;
        }
        match &m.content {
            crate::types::claude::MessageContent::Text { content } => {
                if !content.is_empty() {
                    parts.push(content.clone());
                }
            }
            crate::types::claude::MessageContent::Blocks { content } => {
                for b in content.iter() {
                    if let crate::types::claude::ContentBlock::Text { text } = b {
                        if !text.is_empty() {
                            parts.push(text.clone());
                        }
                    }
                }
            }
        }
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("\n"))
    }
}

fn convert_tools_chat_to_responses(tools: &Value) -> Vec<Value> {
    if let Some(arr) = tools.as_array() {
        arr.iter()
            .filter_map(|t| {
                if t.get("type").and_then(|v| v.as_str()) != Some("function") {
                    return None;
                }
                let f = t.get("function")?.as_object()?;
                let name = f.get("name")?.as_str()?.to_string();
                let desc = f
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let params = f
                    .get("parameters")
                    .cloned()
                    .unwrap_or_else(|| json!({"type": "object", "properties": {}}));
                Some(json!({
                    "type": "function",
                    "name": name,
                    "description": desc,
                    "strict": false,
                    "parameters": params
                }))
            })
            .collect()
    } else {
        vec![]
    }
}

pub async fn codex_chat_completions(
    State(state): State<CodexApiState>,
    Json(raw): Json<Value>,
) -> Result<Response, ClewdrError> {
    // Parse into OAI params for messages, but keep raw for tools and others
    let oai: OaiCreateMessageParams =
        serde_json::from_value(raw.clone()).map_err(|_| ClewdrError::BadRequest {
            msg: "Invalid JSON body".into(),
        })?;
    let requested_model = oai.model.clone();
    let model = state.state.normalize_model_name(Some(&oai.model));
    let stream = oai.stream.unwrap_or(false);
    let created = unix_time();
    let include_usage = raw
        .get("stream_options")
        .and_then(|v| v.get("include_usage"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let tools = convert_tools_chat_to_responses(raw.get("tools").unwrap_or(&Value::Null));
    let tool_choice = raw
        .get("tool_choice")
        .cloned()
        .unwrap_or_else(|| json!("auto"));
    let parallel_tool_calls = raw
        .get("parallel_tool_calls")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let reasoning = raw.get("reasoning").cloned();

    // System instructions and input
    let instructions = system_instructions(&oai.messages);
    let input_items = state
        .state
        .convert_messages_to_responses_input(&oai.messages);

    // Session id from headers not accessible here; allow caller to set X-Session-Id later if needed
    let upstream = state
        .state
        .start_upstream(
            &model,
            instructions,
            input_items,
            tools,
            tool_choice,
            parallel_tool_calls,
            reasoning,
            None,
        )
        .await?;

    if !upstream.status().is_success() {
        let body = upstream.text().await.unwrap_or_default();
        let v: Value = serde_json::from_str(&body).unwrap_or(json!({"raw": body}));
        let msg = v
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or("Upstream error");
        return Ok((
            axum::http::StatusCode::BAD_GATEWAY,
            Json(json!({"error": {"message": msg}})),
        )
            .into_response());
    }

    if stream {
        let s = upstream.bytes_stream().eventsource();
        let model_out = requested_model.clone();
        let s = s.map_ok(move |event| {
            use axum::response::sse::Event;
            // We'll translate each upstream event into OAI delta chunks.
            let mut out_event = Event::default().event(event.event).id(event.id);
            if let Some(retry) = event.retry { out_event = out_event.retry(retry); }
            // Parse JSON
            let v: Value = match serde_json::from_str(&event.data) { Ok(v) => v, Err(_) => return out_event.data(event.data) };
            let kind = v.get("type").and_then(|v| v.as_str()).unwrap_or("");
            // Track id
            let response_id = v.get("response").and_then(|r| r.get("id")).and_then(|v| v.as_str()).unwrap_or("chatcmpl-stream");
            if kind == "response.output_text.delta" {
                let delta = v.get("delta").and_then(|v| v.as_str()).unwrap_or("");
                let chunk = json!({
                    "id": response_id,
                    "object": "chat.completion.chunk",
                    "created": created,
                    "model": model_out,
                    "choices": [{"index": 0, "delta": {"content": delta}, "finish_reason": Value::Null}],
                });
                return out_event.json_data(chunk).unwrap();
            }
            if kind == "response.output_item.done" {
                // tool call
                let item = v.get("item").cloned().unwrap_or(json!({}));
                if item.get("type").and_then(|v| v.as_str()) == Some("function_call") {
                    let call_id = item.get("call_id").or(item.get("id")).and_then(|v| v.as_str()).unwrap_or("");
                    let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
                    let args = item.get("arguments").and_then(|v| v.as_str()).unwrap_or("");
                    let delta_chunk = json!({
                        "id": response_id,
                        "object": "chat.completion.chunk",
                        "created": created,
                        "model": model_out,
                        "choices": [{
                            "index": 0,
                            "delta": {"tool_calls": [{"index": 0, "id": call_id, "type": "function", "function": {"name": name, "arguments": args}}]},
                            "finish_reason": Value::Null
                        }],
                    });
                    return out_event.json_data(delta_chunk).unwrap();
                }
            }
            if kind == "response.output_text.done" {
                let chunk = json!({
                    "id": response_id,
                    "object": "chat.completion.chunk",
                    "created": created,
                    "model": model_out,
                    "choices": [{"index": 0, "delta": {}, "finish_reason": "stop"}],
                });
                return out_event.json_data(chunk).unwrap();
            }
            if kind == "response.failed" {
                let msg = v.get("response").and_then(|r| r.get("error")).and_then(|e| e.get("message")).and_then(|m| m.as_str()).unwrap_or("response.failed");
                let chunk = json!({"error": {"message": msg}});
                return out_event.json_data(chunk).unwrap();
            }
            if kind == "response.completed" {
                // Include usage chunk if requested
                if include_usage {
                    if let Some(usage) = v.get("response").and_then(|r| r.get("usage")).cloned() {
                        let chunk = json!({
                            "id": response_id,
                            "object": "chat.completion.chunk",
                            "created": created,
                            "model": model_out,
                            "choices": [{"index": 0, "delta": {}, "finish_reason": Value::Null}],
                            "usage": usage,
                        });
                        return out_event.json_data(chunk).unwrap();
                    }
                }
                // fall-through as normal event
            }
            out_event.data(event.data)
        });

        return Ok(Sse::new(s).keep_alive(Default::default()).into_response());
    }

    // Non-stream: aggregate
    let mut full_text = String::new();
    let mut response_id = String::from("chatcmpl");
    let mut usage_out: Option<Value> = None;
    let mut tool_calls: Vec<Value> = vec![];
    let mut stream = upstream.bytes_stream().eventsource();
    while let Some(evt) = stream.try_next().await.unwrap_or(None) {
        let v: Value = match serde_json::from_str(&evt.data) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(id) = v
            .get("response")
            .and_then(|r| r.get("id"))
            .and_then(|v| v.as_str())
        {
            response_id = id.to_string();
        }
        let kind = v.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if kind == "response.output_text.delta" {
            if let Some(d) = v.get("delta").and_then(|v| v.as_str()) {
                full_text.push_str(d);
            }
        } else if kind == "response.output_item.done" {
            let item = v.get("item").cloned().unwrap_or(json!({}));
            if item.get("type").and_then(|v| v.as_str()) == Some("function_call") {
                let call_id = item
                    .get("call_id")
                    .or(item.get("id"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let args = item.get("arguments").and_then(|v| v.as_str()).unwrap_or("");
                tool_calls.push(json!({
                    "id": call_id,
                    "type": "function",
                    "function": {"name": name, "arguments": args}
                }));
            }
        } else if kind == "response.completed" {
            usage_out = v.get("response").and_then(|r| r.get("usage")).cloned();
            break;
        } else if kind == "response.failed" {
            let msg = v
                .get("response")
                .and_then(|r| r.get("error"))
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("response.failed");
            return Ok((
                axum::http::StatusCode::BAD_GATEWAY,
                Json(json!({"error": {"message": msg}})),
            )
                .into_response());
        }
    }
    let mut message = json!({"role": "assistant", "content": full_text});
    if !tool_calls.is_empty() {
        message["tool_calls"] = json!(tool_calls);
    }
    let completion = json!({
        "id": response_id,
        "object": "chat.completion",
        "created": created,
        "model": requested_model,
        "choices": [{"index": 0, "message": message, "finish_reason": "stop"}],
    });
    let completion = if let Some(u) = usage_out {
        merge_with_usage(completion, u)
    } else {
        completion
    };
    Ok((axum::http::StatusCode::OK, Json(completion)).into_response())
}

pub async fn codex_completions(
    State(state): State<CodexApiState>,
    Json(raw): Json<Value>,
) -> Result<Response, ClewdrError> {
    // Support legacy /v1/completions style
    let mut prompt = String::new();
    if let Some(p) = raw.get("prompt") {
        if p.is_string() {
            prompt = p.as_str().unwrap_or("").to_string();
        }
        if p.is_array() {
            prompt = p
                .as_array()
                .unwrap()
                .iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join("");
        }
    }
    if prompt.is_empty() {
        prompt = raw
            .get("suffix")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
    }
    let stream = raw.get("stream").and_then(|v| v.as_bool()).unwrap_or(false);
    let include_usage = raw
        .get("stream_options")
        .and_then(|v| v.get("include_usage"))
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let requested_model = raw
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or("gpt-5")
        .to_string();
    let model = state.state.normalize_model_name(Some(&requested_model));
    let msgs = vec![Message::new_text(Role::User, prompt)];
    let instructions = system_instructions(&msgs);
    let input_items = state.state.convert_messages_to_responses_input(&msgs);

    let upstream = state
        .state
        .start_upstream(
            &model,
            instructions,
            input_items,
            vec![],
            json!("auto"),
            false,
            None,
            None,
        )
        .await?;
    if !upstream.status().is_success() {
        let body = upstream.text().await.unwrap_or_default();
        let v: Value = serde_json::from_str(&body).unwrap_or(json!({"raw": body}));
        let msg = v
            .get("error")
            .and_then(|e| e.get("message"))
            .and_then(|v| v.as_str())
            .unwrap_or("Upstream error");
        return Ok((
            axum::http::StatusCode::BAD_GATEWAY,
            Json(json!({"error": {"message": msg}})),
        )
            .into_response());
    }

    let created = unix_time();
    if stream {
        let model_out = requested_model.clone();
        let s = upstream.bytes_stream().eventsource().map_ok(move |event| {
            use axum::response::sse::Event;
            let mut out_event = Event::default().event(event.event).id(event.id);
            if let Some(retry) = event.retry {
                out_event = out_event.retry(retry);
            }
            let v: Value = match serde_json::from_str(&event.data) {
                Ok(v) => v,
                Err(_) => return out_event.data(event.data),
            };
            let response_id = v
                .get("response")
                .and_then(|r| r.get("id"))
                .and_then(|v| v.as_str())
                .unwrap_or("cmpl-stream");
            let kind = v.get("type").and_then(|v| v.as_str()).unwrap_or("");
            if kind == "response.output_text.delta" {
                let delta = v.get("delta").and_then(|v| v.as_str()).unwrap_or("");
                let chunk = json!({
                    "id": response_id,
                    "object": "text_completion.chunk",
                    "created": created,
                    "model": model_out,
                    "choices": [{"index": 0, "text": delta, "finish_reason": Value::Null}],
                });
                return out_event.json_data(chunk).unwrap();
            }
            if kind == "response.output_text.done" {
                let chunk = json!({
                    "id": response_id,
                    "object": "text_completion.chunk",
                    "created": created,
                    "model": model_out,
                    "choices": [{"index": 0, "text": "", "finish_reason": "stop"}],
                });
                return out_event.json_data(chunk).unwrap();
            }
            if kind == "response.completed" && include_usage {
                if let Some(usage) = v.get("response").and_then(|r| r.get("usage")).cloned() {
                    let chunk = json!({
                        "id": response_id,
                        "object": "text_completion.chunk",
                        "created": created,
                        "model": model_out,
                        "choices": [{"index": 0, "text": "", "finish_reason": Value::Null}],
                        "usage": usage,
                    });
                    return out_event.json_data(chunk).unwrap();
                }
            }
            out_event.data(event.data)
        });
        return Ok(Sse::new(s).keep_alive(Default::default()).into_response());
    }

    // aggregate
    let mut full_text = String::new();
    let mut response_id = String::from("cmpl");
    let mut usage_out: Option<Value> = None;
    let mut stream = upstream.bytes_stream().eventsource();
    while let Some(evt) = stream.try_next().await.unwrap_or(None) {
        let v: Value = match serde_json::from_str(&evt.data) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if let Some(id) = v
            .get("response")
            .and_then(|r| r.get("id"))
            .and_then(|v| v.as_str())
        {
            response_id = id.to_string();
        }
        let kind = v.get("type").and_then(|v| v.as_str()).unwrap_or("");
        if kind == "response.output_text.delta" {
            if let Some(d) = v.get("delta").and_then(|v| v.as_str()) {
                full_text.push_str(d);
            }
        } else if kind == "response.completed" {
            usage_out = v.get("response").and_then(|r| r.get("usage")).cloned();
            break;
        } else if kind == "response.failed" {
            let msg = v
                .get("response")
                .and_then(|r| r.get("error"))
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
                .unwrap_or("response.failed");
            return Ok((
                axum::http::StatusCode::BAD_GATEWAY,
                Json(json!({"error": {"message": msg}})),
            )
                .into_response());
        }
    }

    let completion = json!({
        "id": response_id,
        "object": "text_completion",
        "created": unix_time(),
        "model": requested_model,
        "choices": [{"index": 0, "text": full_text, "finish_reason": "stop", "logprobs": Value::Null}],
    });
    let completion = if let Some(u) = usage_out {
        merge_with_usage(completion, u)
    } else {
        completion
    };
    Ok((axum::http::StatusCode::OK, Json(completion)).into_response())
}

pub async fn codex_list_models() -> impl IntoResponse {
    let data = vec![
        json!({"id": "gpt-5", "object": "model", "owned_by": "owner"}),
        json!({"id": "codex-mini-latest", "object": "model", "owned_by": "owner"}),
    ];
    (
        axum::http::StatusCode::OK,
        Json(json!({"object": "list", "data": data})),
    )
}

fn merge_with_usage(mut obj: Value, usage: Value) -> Value {
    obj["usage"] = usage;
    obj
}
