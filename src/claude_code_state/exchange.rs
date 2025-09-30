use oauth2::PkceCodeVerifier;

use crate::{
    claude_code_state::ClaudeCodeState,
    config::{CLEWDR_CONFIG, CookieStatus},
    error::ClewdrError,
};

pub struct ExchangeResult {
    code: String,
    state: Option<String>,
    verifier: PkceCodeVerifier,
    org_uuid: String,
}

impl ClaudeCodeState {
    pub async fn exchange_code(&self, org_uuid: &str) -> Result<ExchangeResult, ClewdrError> {
        let cc_client_id = CLEWDR_CONFIG.load().cc_client_id();
        let res = crate::anthropic::oauth::exchange_code(&self.client, &self.endpoint, org_uuid, cc_client_id)
            .await?;
        Ok(ExchangeResult { code: res.code, state: res.state, verifier: res.verifier, org_uuid: org_uuid.to_string() })
    }

    pub async fn exchange_token(&mut self, code_res: ExchangeResult) -> Result<(), ClewdrError> {
        let cc_client_id = CLEWDR_CONFIG.load().cc_client_id();
        let ti = crate::anthropic::oauth::exchange_token(&self.client, cc_client_id, crate::anthropic::oauth::ExchangeCodeResult { code: code_res.code, state: code_res.state, verifier: code_res.verifier }, code_res.org_uuid.clone()).await?;
        if let Some(cookie) = self.cookie.as_mut() {
            cookie.token = Some(ti);
        } else {
            return Err(ClewdrError::UnexpectedNone {
                msg: "No cookie found to update with token info",
            });
        }
        Ok(())
    }

    pub async fn refresh_token(&mut self) -> Result<(), ClewdrError> {
        let Some(CookieStatus {
            token: Some(ref mut token),
            ..
        }) = self.cookie
        else {
            return Err(ClewdrError::UnexpectedNone {
                msg: "No token found to refresh token",
            });
        };
        if !token.is_expired() {
            return Ok(());
        }

        let cc_client_id = CLEWDR_CONFIG.load().cc_client_id();
        let new_token = crate::anthropic::oauth::refresh_token(&self.client, cc_client_id, token.refresh_token.clone(), token.organization.uuid.clone()).await?;
        *token = new_token;
        Ok(())
    }

}
