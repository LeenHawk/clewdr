pub mod admin;
pub mod claude_code;
pub mod claude_web;
pub mod codex;
pub mod codex_oauth;
pub mod gemini;
pub mod gemini_cli;

pub use admin::build_admin_router;
pub use claude_code::{build_claude_code_oai_router, build_claude_code_router};
pub use claude_web::{build_claude_web_oai_router, build_claude_web_router};
pub use codex::build_codex_router;
pub use codex_oauth::build_codex_oauth_router;
pub use gemini::build_gemini_router;
pub use gemini_cli::build_gemini_cli_router;
