use std::sync::{Arc, Mutex};

use axum::{
    Json,
    extract::Query,
    response::{Html, IntoResponse},
};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use chrono::Utc;
use http::header::CONTENT_TYPE;
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use tracing::{error, info};
use url::{Url, form_urlencoded};
use wreq::{Client, ClientBuilder, Method};

use crate::config::{CLEWDR_CONFIG, CodexTokens};

#[derive(Debug, Clone)]
struct PendingOauth {
    state: String,
    code_verifier: String,
    redirect_uri: String,
}

static PENDING: once_cell::sync::Lazy<Arc<Mutex<Option<PendingOauth>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(None)));

#[derive(Serialize)]
pub struct StartAuthResponse {
    pub auth_url: String,
}

/// GET /api/codex/oauth/start (admin)
/// Returns an authorization URL and stores PKCE/state in-memory for the callback.
pub async fn api_codex_oauth_start() -> impl IntoResponse {
    // Build redirect uri to our running server (prefix configurable for reverse proxies)
    let cfg = CLEWDR_CONFIG.load();
    let default_prefix = format!("http://localhost:{}", cfg.address().port());
    let prefix = cfg
        .codex
        .oauth_redirect_prefix
        .clone()
        .unwrap_or(default_prefix);
    let prefix = prefix.trim_end_matches('/');
    let redirect_uri = format!("{}/codex/oauth/callback", prefix);

    // Generate PKCE
    let code_verifier: String = rand_hex(64);
    let code_challenge = code_challenge_s256(&code_verifier);
    let state: String = rand_hex(32);

    let client_id = CLEWDR_CONFIG.load().codex.effective_client_id();
    let issuer = crate::config::CODEX_OAUTH_ISSUER;
    let mut auth_url = Url::parse(&format!("{}/oauth/authorize", issuer)).expect("valid issuer");
    auth_url
        .query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("scope", "openid profile email offline_access")
        .append_pair("code_challenge", &code_challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("id_token_add_organizations", "true")
        .append_pair("codex_cli_simplified_flow", "true")
        .append_pair("state", &state);

    // Save pending in memory
    {
        let mut guard = PENDING.lock().unwrap();
        *guard = Some(PendingOauth {
            state: state.clone(),
            code_verifier: code_verifier.clone(),
            redirect_uri: redirect_uri.clone(),
        });
    }

    info!(
        "Codex OAuth start: state set; redirect_uri={}",
        redirect_uri
    );
    Json(StartAuthResponse {
        auth_url: auth_url.to_string(),
    })
}

#[derive(Deserialize)]
pub struct CallbackQuery {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

/// GET /codex/oauth/callback
/// Handles OAuth callback, exchanges code, persists tokens, shows a simple result page.
#[axum::debug_handler]
pub async fn api_codex_oauth_callback(q: Query<CallbackQuery>) -> Html<String> {
    let q = q.0;
    if let Some(err) = q.error.as_deref() {
        let desc = q.error_description.as_deref().unwrap_or("");
        return Html(format!(
            "<html><body><h2>Login error</h2><p>{}</p><p>{}</p></body></html>",
            html_escape(err),
            html_escape(desc)
        ));
    }
    let (code, state) = match (q.code.clone(), q.state.clone()) {
        (Some(c), Some(s)) => (c, s),
        _ => {
            return Html("<html><body><h2>Invalid callback</h2></body></html>".to_string());
        }
    };

    let pending = { PENDING.lock().unwrap().clone() };
    let Some(p) = pending else {
        return Html(
            "<html><body><h2>No pending login or it expired</h2></body></html>".to_string(),
        );
    };
    if p.state != state {
        return Html("<html><body><h2>State mismatch</h2></body></html>".to_string());
    }

    // Exchange code for tokens
    let issuer = crate::config::CODEX_OAUTH_ISSUER;
    let token_url = format!("{}/oauth/token", issuer);
    let client_id = CLEWDR_CONFIG.load().codex.effective_client_id();
    let form = [
        ("grant_type", "authorization_code"),
        ("code", &code),
        ("redirect_uri", &p.redirect_uri),
        ("client_id", &client_id),
        ("code_verifier", &p.code_verifier),
    ];

    let client = http_client();
    let body = {
        let mut enc = form_urlencoded::Serializer::new(String::new());
        for (k, v) in form {
            enc.append_pair(k, v);
        }
        enc.finish()
    };
    let resp = match client
        .request(Method::POST, &token_url)
        .header(CONTENT_TYPE, "application/x-www-form-urlencoded")
        .body(body)
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            error!("Token exchange failed: {}", e);
            return Html(format!(
                "<html><body><h2>Token exchange failed</h2><p>{}</p></body></html>",
                html_escape(&e.to_string())
            ));
        }
    };

    let status = resp.status();
    let body = match resp.text().await {
        Ok(t) => t,
        Err(e) => {
            error!("Failed reading token response: {}", e);
            String::new()
        }
    };
    if !status.is_success() {
        error!("Token endpoint returned {}: {}", status.as_u16(), body);
        return Html(format!(
            "<html><body><h2>Token endpoint error {}</h2><pre>{}</pre></body></html>",
            status.as_u16(),
            html_escape(&body)
        ));
    }

    let payload: serde_json::Value = serde_json::from_str(&body).unwrap_or(json!({}));
    let id_token = payload
        .get("id_token")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let access_token = payload
        .get("access_token")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let refresh_token = payload
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Extract account_id from id_token claims if present
    let account_id = parse_jwt_claim(
        &id_token,
        "https://api.openai.com/auth",
        "chatgpt_account_id",
    );

    // Persist tokens to config
    crate::config::CLEWDR_CONFIG.rcu(|conf| {
        let mut c = crate::config::ClewdrConfig::clone(conf);
        c.codex.tokens = CodexTokens {
            id_token: some_if_not_empty(id_token.clone()),
            access_token: some_if_not_empty(access_token.clone()),
            refresh_token: some_if_not_empty(refresh_token.clone()),
            account_id: option_if_not_empty(account_id.clone()),
            last_refresh: Some(Utc::now().to_rfc3339()),
            api_key: c.codex.tokens.api_key.clone(),
        };
        c
    });
    if let Err(e) = CLEWDR_CONFIG.load().save().await {
        error!("Failed to save config: {}", e);
    }

    // clear pending
    {
        let mut guard = PENDING.lock().unwrap();
        *guard = None;
    }

    Html(
        "<html><body><h2>Login successful</h2><p>You can close this window.</p></body></html>"
            .to_string(),
    )
}

/// GET /api/codex/tokens (admin)
pub async fn api_codex_tokens() -> impl IntoResponse {
    let c = CLEWDR_CONFIG.load();
    let tokens = &c.codex.tokens;
    Json(json!({
        "authenticated": c.codex.is_authenticated(),
        "account_id": tokens.account_id,
        "has_access_token": tokens.access_token.as_ref().map(|s| !s.is_empty()).unwrap_or(false),
        "last_refresh": tokens.last_refresh,
    }))
}

/// POST /api/codex/logout (admin)
pub async fn api_codex_logout() -> impl IntoResponse {
    crate::config::CLEWDR_CONFIG.rcu(|conf| {
        let mut c = crate::config::ClewdrConfig::clone(conf);
        c.codex.tokens = CodexTokens::default();
        c
    });
    if let Err(e) = CLEWDR_CONFIG.load().save().await {
        error!("Failed to save config: {}", e);
    }
    Json(json!({"ok": true}))
}

fn http_client() -> Client {
    let mut builder = ClientBuilder::new();
    if let Some(p) = &CLEWDR_CONFIG.load().wreq_proxy {
        builder = builder.proxy(p.to_owned());
    }
    builder.build().unwrap_or_else(|_| Client::new())
}

fn rand_hex(nbytes: usize) -> String {
    // Generate nbytes of random data and hex-encode to 2*nbytes length
    let mut buf = vec![0u8; nbytes];
    rand::thread_rng().fill(&mut buf[..]);
    hex::encode(buf)
}

fn code_challenge_s256(verifier: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(verifier.as_bytes());
    let digest = hasher.finalize();
    URL_SAFE_NO_PAD.encode(digest)
}

fn parse_jwt_claim(token: &str, top_ns: &str, key: &str) -> Option<String> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload = parts[1];
    let decoded = URL_SAFE_NO_PAD.decode(payload.as_bytes()).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&decoded).ok()?;
    v.get(top_ns)
        .and_then(|ns| ns.get(key))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn html_escape(s: &str) -> String {
    htmlescape::encode_minimal(s)
}

fn some_if_not_empty(s: String) -> Option<String> {
    let t = s.trim();
    if t.is_empty() { None } else { Some(s) }
}

fn option_if_not_empty(s: Option<String>) -> Option<String> {
    s.and_then(|v| if v.trim().is_empty() { None } else { Some(v) })
}
