use axum::{
    Router,
    middleware::from_extractor,
    routing::{get, post},
};
use tower::ServiceBuilder;
use tower_http::compression::CompressionLayer;

use crate::{api::*, codex_state::CodexState, middleware::RequireBearerAuth};

pub fn build_codex_router() -> Router {
    let state = crate::api::codex::CodexApiState {
        state: CodexState::new(),
    };
    Router::new()
        .route("/codex/v1/chat/completions", post(codex_chat_completions))
        .route("/codex/v1/completions", post(codex_completions))
        .route("/codex/v1/models", get(codex_list_models))
        .layer(
            ServiceBuilder::new()
                .layer(from_extractor::<RequireBearerAuth>())
                .layer(CompressionLayer::new()),
        )
        .with_state(state)
}
