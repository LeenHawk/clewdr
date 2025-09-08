/// API module for handling all external HTTP endpoints and request/response transformations
///
/// This module serves as the main entry point for all API requests, providing endpoints
/// for configuration management, message handling, authentication, and OpenAI-compatible
/// interfaces. It also implements response transformation between different API formats.
mod claude_code;
mod claude_web;
pub mod codex;
mod codex_oauth;
mod config;
mod error;
mod gemini;
mod gemini_cli;
mod misc;
mod storage;
pub use claude_code::api_claude_code;
/// Message handling endpoints for creating and managing chat conversations
pub use claude_web::api_claude_web;
pub use codex::{codex_chat_completions, codex_completions, codex_list_models};
pub use codex_oauth::{
    api_codex_logout, api_codex_oauth_callback, api_codex_oauth_start, api_codex_tokens,
};
/// Configuration related endpoints for retrieving and updating Clewdr settings
pub use config::{api_get_config, api_post_config};
pub use error::ApiError;
pub use gemini::{api_post_gemini, api_post_gemini_oai};
pub use gemini_cli::{
    api_gemini_cli_model_info, api_gemini_cli_models, api_post_gemini_cli, api_post_gemini_cli_oai,
};
/// Miscellaneous endpoints for authentication, cookies, and version information
pub use misc::{
    api_auth, api_delete_cli_token, api_delete_cookie, api_delete_key, api_get_cli_tokens,
    api_get_cookies, api_get_keys, api_get_models, api_post_cli_token, api_post_cookie,
    api_post_key, api_version,
};
pub use storage::{api_storage_export, api_storage_import, api_storage_status};
// merged above
