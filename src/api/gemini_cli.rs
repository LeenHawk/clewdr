use axum::{Json, extract::Path};
use serde_json::{Value, json};

use crate::error::ClewdrError;

#[allow(dead_code)]
fn gemini_cli_model_info(name: &str) -> Value {
    json!({
        "name": format!("models/{}", name),
        "baseModelId": name,
        "version": "001",
        "displayName": name,
        "description": format!("Gemini {} model", name),
        "inputTokenLimit": 128_000,
        "outputTokenLimit": 8_192,
        "supportedGenerationMethods": ["generateContent", "streamGenerateContent"],
        "temperature": 1.0,
        "maxTemperature": 2.0,
        "topP": 0.95,
        "topK": 64,
    })
}

pub async fn api_gemini_cli_models() -> Result<Json<Value>, ClewdrError> {
    let models = ["gemini-2.5-pro", "gemini-2.5-flash"];
    let items: Vec<_> = models.iter().map(|m| gemini_cli_model_info(m)).collect();
    Ok(Json(json!({ "models": items })))
}

pub async fn api_gemini_cli_model_info(
    Path(path): Path<String>,
) -> Result<Json<Value>, ClewdrError> {
    let id = path.strip_prefix("models/").unwrap_or(path.as_str());
    Ok(Json(gemini_cli_model_info(id)))
}
