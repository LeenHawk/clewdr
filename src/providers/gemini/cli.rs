use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use chrono::{Duration, Utc};
use tokio::sync::Mutex;
use tracing::{error, info};
use wreq::ClientBuilder;

use crate::{
    config::{CLEWDR_CONFIG, ClewdrConfig, GeminiCliCredential},
    error::ClewdrError,
    gemini_state::GeminiState,
    middleware::gemini::GeminiContext,
    providers::LLMProvider,
    services::key_actor::KeyActorHandle,
};

use super::{GeminiInvocation, GeminiPayload};

pub struct GeminiCliProvider {
    key_actor_handle: KeyActorHandle,
    credentials: Arc<GeminiCliCredentialPool>,
}

impl GeminiCliProvider {
    pub fn new(
        key_actor_handle: KeyActorHandle,
        credentials: Arc<GeminiCliCredentialPool>,
    ) -> Self {
        Self {
            key_actor_handle,
            credentials,
        }
    }

    fn build_state(&self, ctx: &GeminiContext) -> Result<GeminiState, ClewdrError> {
        if ctx.vertex {
            return Err(ClewdrError::BadRequest {
                msg: "Vertex request routed to Gemini CLI provider",
            });
        }
        let mut state = GeminiState::new(self.key_actor_handle.clone());
        state.update_from_ctx(ctx);
        state.cli = true;
        Ok(state)
    }

    async fn ensure_token(
        &self,
        credential: GeminiCliCredential,
        force: bool,
    ) -> Result<GeminiCliCredential, ClewdrError> {
        if !force && !credential.needs_refresh() {
            return Ok(credential);
        }
        let identifier = credential.identifier();
        let lock = self.credentials.lock(&identifier).await;
        let _guard = lock.lock().await;

        let refreshed_snapshot = self
            .credentials
            .get(&identifier)
            .unwrap_or_else(|| credential.clone());
        if !force && !refreshed_snapshot.needs_refresh() {
            return Ok(refreshed_snapshot);
        }

        let updated = refresh_cli_token(&refreshed_snapshot).await?;
        persist_cli_credential(&updated).await?;
        Ok(updated)
    }
}

#[async_trait::async_trait]
impl LLMProvider for GeminiCliProvider {
    type Request = GeminiInvocation;
    type Output = axum::response::Response;

    async fn invoke(&self, request: Self::Request) -> Result<Self::Output, ClewdrError> {
        if !request.context.cli {
            return Err(ClewdrError::BadRequest {
                msg: "AI Studio or Vertex request routed to Gemini CLI provider",
            });
        }

        let mut state = self.build_state(&request.context)?;
        let Some(mut credential) = self.credentials.next() else {
            return Err(ClewdrError::BadRequest {
                msg: "Gemini CLI credential not found",
            });
        };
        credential = self.ensure_token(credential, false).await?;
        state.set_cli_credential(credential.clone());

        match request.payload {
            GeminiPayload::Native(body) => {
                let mut attempt_state = state;
                let first_attempt = attempt_state.try_chat(body.clone()).await;
                match first_attempt {
                    Ok(resp) => Ok(resp),
                    Err(ClewdrError::GeminiHttpError { code, .. })
                        if code == 401 || code == 403 =>
                    {
                        let refreshed = self.ensure_token(credential, true).await?;
                        let mut retry_state = self.build_state(&request.context)?;
                        retry_state.set_cli_credential(refreshed);
                        retry_state.try_chat(body).await
                    }
                    Err(err) => Err(err),
                }
            }
            GeminiPayload::OpenAI(_) => Err(ClewdrError::BadRequest {
                msg: "OpenAI format is not supported on the Gemini CLI endpoint",
            }),
        }
    }
}

#[derive(Default)]
pub struct GeminiCliCredentialPool {
    cursor: AtomicUsize,
    locks: Mutex<HashMap<String, Arc<Mutex<()>>>>,
}

impl GeminiCliCredentialPool {
    pub fn next(&self) -> Option<GeminiCliCredential> {
        let creds = CLEWDR_CONFIG.load().gemini_cli_credentials.clone();
        if creds.is_empty() {
            return None;
        }
        let idx = self.cursor.fetch_add(1, Ordering::Relaxed);
        Some(creds[idx % creds.len()].clone())
    }

    pub fn get(&self, identifier: &str) -> Option<GeminiCliCredential> {
        CLEWDR_CONFIG
            .load()
            .gemini_cli_credentials
            .iter()
            .find(|cred| cred.identifier() == identifier)
            .cloned()
    }

    pub async fn lock(&self, identifier: &str) -> Arc<Mutex<()>> {
        let mut guard = self.locks.lock().await;
        guard
            .entry(identifier.to_string())
            .or_insert_with(|| Arc::new(Mutex::new(())))
            .clone()
    }
}

async fn refresh_cli_token(
    credential: &GeminiCliCredential,
) -> Result<GeminiCliCredential, ClewdrError> {
    let mut client = ClientBuilder::new();
    if let Some(proxy) = CLEWDR_CONFIG.load().wreq_proxy.to_owned() {
        client = client.proxy(proxy);
    }
    let client = client.build().map_err(|source| ClewdrError::WreqError {
        msg: "Failed to build OAuth client for Gemini CLI",
        source,
    })?;

    let mut form = HashMap::new();
    form.insert("client_id", credential.client_id.to_string());
    form.insert("client_secret", credential.client_secret.to_string());
    form.insert("refresh_token", credential.refresh_token.to_string());
    form.insert("grant_type", "refresh_token".to_string());
    if !credential.scopes.is_empty() {
        form.insert("scope", credential.scopes.join(" "));
    }

    let response = client
        .post(&credential.token_uri)
        .form(&form)
        .send()
        .await
        .map_err(|source| ClewdrError::WreqError {
            msg: "Failed to refresh Gemini CLI token",
            source,
        })?;

    let status = response.status();
    let value: serde_json::Value =
        response
            .json()
            .await
            .map_err(|source| ClewdrError::WreqError {
                msg: "Failed to parse Gemini CLI token response",
                source,
            })?;

    if !status.is_success() {
        error!("Failed to refresh Gemini CLI token: {}", value);
        return Err(ClewdrError::BadRequest {
            msg: "Unable to refresh Gemini CLI credential",
        });
    }

    let access_token =
        value
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or(ClewdrError::BadRequest {
                msg: "Missing access token in refresh response",
            })?;
    let expires_in = value
        .get("expires_in")
        .and_then(|v| v.as_i64())
        .unwrap_or(3600);
    let scope = value.get("scope").and_then(|v| v.as_str()).map(|s| {
        s.split_whitespace()
            .map(|x| x.to_string())
            .collect::<Vec<_>>()
    });

    let expiry = Utc::now() + Duration::seconds(expires_in.max(30) - 30);
    info!(
        "Gemini CLI token refreshed for project: {}",
        credential.project_id
    );
    Ok(credential.with_token(access_token.to_string(), Some(expiry), scope))
}

async fn persist_cli_credential(credential: &GeminiCliCredential) -> Result<(), ClewdrError> {
    CLEWDR_CONFIG.rcu(|config| {
        let mut new_config = ClewdrConfig::clone(config);
        new_config
            .gemini_cli_credentials
            .retain(|cred| cred.identifier() != credential.identifier());
        new_config.gemini_cli_credentials.push(credential.clone());
        new_config = new_config.validate();
        new_config
    });

    CLEWDR_CONFIG.load().save().await?;
    Ok(())
}
