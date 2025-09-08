use axum::{
    Router,
    middleware::from_extractor,
    routing::{get, post},
};

use crate::{api::*, middleware::RequireAdminAuth};

pub fn build_codex_oauth_router() -> Router {
    let admin = Router::new()
        .route("/api/codex/oauth/start", get(api_codex_oauth_start))
        .route("/api/codex/tokens", get(api_codex_tokens))
        .route("/api/codex/logout", post(api_codex_logout))
        .layer(from_extractor::<RequireAdminAuth>())
        .with_state(());

    Router::new()
        .route("/codex/oauth/callback", get(api_codex_oauth_callback))
        .merge(admin)
        .with_state(())
}
