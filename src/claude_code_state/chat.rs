use axum::response::{Sse, sse::Event as SseEvent};
use axum::{Json, response::IntoResponse};
use colored::Colorize;
use eventsource_stream::Eventsource;
use futures::TryStreamExt;
use snafu::{GenerateImplicitData, ResultExt};
use tracing::{Instrument, error, info, warn};

use crate::{
    claude_code_state::{ClaudeCodeState, TokenStatus},
    config::{ModelFamily, CLEWDR_CONFIG, CLAUDE_CONSOLE_ENDPOINT},
    error::{CheckClaudeErr, ClewdrError, WreqSnafu},
    types::claude::{CountMessageTokensResponse, CreateMessageParams},
};
use wreq::Url;

const CLAUDE_BETA_BASE: &str = "oauth-2025-04-20";
const CLAUDE_BETA_CONTEXT_1M: &str = "oauth-2025-04-20,context-1m-2025-08-07";

impl ClaudeCodeState {
    /// Attempts to send a chat message to Claude API with retry mechanism
    ///
    /// This method handles the complete chat flow including:
    /// - Request preparation and logging
    /// - Cookie management for authentication
    /// - Executing the chat request with automatic retries on failure
    /// - Response transformation according to the specified API format
    /// - Error handling and cleanup
    ///
    /// The method implements a sophisticated retry mechanism to handle transient failures,
    /// and manages conversation cleanup to prevent resource leaks. It also includes
    /// performance tracking to measure response times.
    ///
    /// # Arguments
    /// * `p` - The client request body containing messages and configuration
    ///
    /// # Returns
    /// * `Result<axum::response::Response, ClewdrError>` - Formatted response or error
    pub async fn try_chat(
        &mut self,
        p: CreateMessageParams,
    ) -> Result<axum::response::Response, ClewdrError> {
        for i in 0..CLEWDR_CONFIG.load().max_retries + 1 {
            if i > 0 {
                info!("[RETRY] attempt: {}", i.to_string().green());
            }
            let mut state = self.to_owned();
            let p = p.to_owned();

            let cookie = state.request_cookie().await?;
            let retry = async {
                match state.check_token() {
                    TokenStatus::None => {
                        info!("No token found, requesting new token");
                        let org = state.get_organization().await?;
                        let code_res = state.exchange_code(&org).await?;
                        state.exchange_token(code_res).await?;
                        state.return_cookie(None).await;
                    }
                    TokenStatus::Expired => {
                        info!("Token expired, refreshing token");
                        state.refresh_token().await?;
                        state.return_cookie(None).await;
                    }
                    TokenStatus::Valid => {
                        info!("Token is valid, proceeding with request");
                    }
                }
                let Some(access_token) = state.cookie.as_ref().and_then(|c| c.token.to_owned())
                else {
                    return Err(ClewdrError::UnexpectedNone {
                        msg: "No access token found in cookie",
                    });
                };
                state
                    .send_chat(access_token.access_token.to_owned(), p)
                    .await
            }
            .instrument(tracing::info_span!(
                "claude_code",
                "cookie" = cookie.cookie.ellipse()
            ));
            match retry.await {
                Ok(res) => {
                    return Ok(res);
                }
                Err(e) => {
                    error!(
                        "[{}] {}",
                        state.cookie.as_ref().unwrap().cookie.ellipse().green(),
                        e
                    );
                    // 429 error
                    if let ClewdrError::InvalidCookie { reason } = e {
                        state.return_cookie(Some(reason.to_owned())).await;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        Err(ClewdrError::TooManyRetries)
    }

    pub async fn send_chat(
        &mut self,
        access_token: String,
        mut p: CreateMessageParams,
    ) -> Result<axum::response::Response, ClewdrError> {
        let (base_model, requested_1m) = match p.model.strip_suffix("-1M") {
            Some(stripped) => (stripped.to_string(), true),
            None => (p.model.clone(), false),
        };

        let is_sonnet = Self::is_sonnet4_model(&base_model);
        let cookie_support = self
            .cookie
            .as_ref()
            .and_then(|cookie| cookie.supports_claude_1m);

        let attempts: Vec<bool> = if is_sonnet {
            match cookie_support {
                Some(true) => vec![true],
                Some(false) => vec![false],
                None => vec![true, false],
            }
        } else if requested_1m {
            vec![true, false]
        } else {
            vec![false]
        };

        p.model = base_model;
        let model_family = Self::classify_model(&p.model);

        let mut last_err: Option<ClewdrError> = None;
        for (idx, use_1m) in attempts.iter().copied().enumerate() {
            match self.execute_claude_request(&access_token, &p, use_1m).await {
                Ok(response) => {
                    return self
                        .handle_success_response(response, is_sonnet && use_1m, model_family)
                        .await;
                }
                Err(err) => {
                    let is_last_attempt = idx + 1 == attempts.len();
                    let should_retry = use_1m
                        && is_sonnet
                        && !is_last_attempt
                        && Self::is_context_1m_forbidden(&err);

                    if should_retry {
                        warn!(
                            "1M context not available for current cookie, disabling automatic 1M attempts"
                        );
                        self.persist_claude_1m_support(false).await;
                        last_err = Some(err);
                        continue;
                    }
                    return Err(err);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| ClewdrError::TooManyRetries))
    }

    async fn execute_claude_request(
        &mut self,
        access_token: &str,
        body: &CreateMessageParams,
        use_context_1m: bool,
    ) -> Result<wreq::Response, ClewdrError> {
        let beta_header = if use_context_1m {
            CLAUDE_BETA_CONTEXT_1M
        } else {
            CLAUDE_BETA_BASE
        };

        self.client
            .post(format!("{}/v1/messages", self.endpoint))
            .bearer_auth(access_token)
            .header("anthropic-beta", beta_header)
            .header("anthropic-version", "2023-06-01")
            .json(body)
            .send()
            .await
            .context(WreqSnafu {
                msg: "Failed to send chat message",
            })?
            .check_claude()
            .await
    }

    async fn persist_claude_1m_support(&mut self, value: bool) {
        if let Some(cookie) = self.cookie.as_mut() {
            if cookie.supports_claude_1m == Some(value) {
                return;
            }
            cookie.set_claude_1m_support(Some(value));
            let cloned = cookie.clone();
            if let Err(err) = self.cookie_actor_handle.return_cookie(cloned, None).await {
                warn!("Failed to persist Claude 1M support state: {}", err);
            }
        }
    }

    async fn persist_count_tokens_allowed(&mut self, value: bool) {
        if let Some(cookie) = self.cookie.as_mut() {
            if cookie.count_tokens_allowed == Some(value) {
                return;
            }
            cookie.set_count_tokens_allowed(Some(value));
            let cloned = cookie.clone();
            if let Err(err) = self.cookie_actor_handle.return_cookie(cloned, None).await {
                warn!("Failed to persist count_tokens permission: {}", err);
            }
        }
    }

    pub async fn try_count_tokens(
        &mut self,
        p: CreateMessageParams,
        for_web: bool,
    ) -> Result<axum::response::Response, ClewdrError> {
        for i in 0..CLEWDR_CONFIG.load().max_retries + 1 {
            if i > 0 {
                info!("[TOKENS][RETRY] attempt: {}", i.to_string().green());
            }
            let mut state = self.to_owned();
            let p = p.to_owned();

            let cookie = state.request_cookie().await?;
            let web_attempt_allowed = CLEWDR_CONFIG.load().enable_web_count_tokens;
            let cookie_disallows = matches!(cookie.count_tokens_allowed, Some(false));
            if cookie_disallows || (for_web && !web_attempt_allowed) {
                if cookie_disallows {
                    state.persist_count_tokens_allowed(false).await;
                }
                return Ok(Self::local_count_tokens_response(&p));
            }
            let retry = async {
                match state.check_token() {
                    TokenStatus::None => {
                        info!("No token found, requesting new token");
                        let org = state.get_organization().await?;
                        let code_res = state.exchange_code(&org).await?;
                        state.exchange_token(code_res).await?;
                        state.return_cookie(None).await;
                    }
                    TokenStatus::Expired => {
                        info!("Token expired, refreshing token");
                        state.refresh_token().await?;
                        state.return_cookie(None).await;
                    }
                    TokenStatus::Valid => {
                        info!("Token is valid, proceeding with count_tokens");
                    }
                }
                let Some(access_token) = state.cookie.as_ref().and_then(|c| c.token.to_owned())
                else {
                    return Err(ClewdrError::UnexpectedNone {
                        msg: "No access token found in cookie",
                    });
                };
                state
                    .perform_count_tokens(access_token.access_token.to_owned(), p, for_web)
                    .await
            }
            .instrument(tracing::info_span!(
                "claude_code_tokens",
                "cookie" = cookie.cookie.ellipse()
            ));
            match retry.await {
                Ok(res) => {
                    return Ok(res);
                }
                Err(e) => {
                    error!(
                        "[{}][TOKENS] {}",
                        state.cookie.as_ref().unwrap().cookie.ellipse().green(),
                        e
                    );
                    if let ClewdrError::InvalidCookie { reason } = e {
                        state.return_cookie(Some(reason.to_owned())).await;
                        continue;
                    }
                    return Err(e);
                }
            }
        }
        Err(ClewdrError::TooManyRetries)
    }

    async fn perform_count_tokens(
        &mut self,
        access_token: String,
        mut p: CreateMessageParams,
        allow_fallback: bool,
    ) -> Result<axum::response::Response, ClewdrError> {
        p.stream = Some(false);
        let (base_model, requested_1m) = match p.model.strip_suffix("-1M") {
            Some(stripped) => (stripped.to_string(), true),
            None => (p.model.clone(), false),
        };

        let is_sonnet = Self::is_sonnet4_model(&base_model);
        let cookie_support = self
            .cookie
            .as_ref()
            .and_then(|cookie| cookie.supports_claude_1m);

        let attempts: Vec<bool> = if is_sonnet {
            match cookie_support {
                Some(true) => vec![true],
                Some(false) => vec![false],
                None => vec![true, false],
            }
        } else if requested_1m {
            vec![true, false]
        } else {
            vec![false]
        };

        p.model = base_model;

        let mut last_err: Option<ClewdrError> = None;
        for (idx, use_1m) in attempts.iter().copied().enumerate() {
            match self
                .execute_claude_count_tokens_request(&access_token, &p, use_1m)
                .await
            {
                Ok(response) => {
                    self.persist_count_tokens_allowed(true).await;
                    if is_sonnet && use_1m {
                        self.persist_claude_1m_support(true).await;
                    }
                    let (resp, _) = Self::materialize_non_stream_response(response).await?;
                    return Ok(resp);
                }
                Err(err) => {
                    let unauthorized = Self::is_count_tokens_unauthorized(&err);
                    if unauthorized {
                        self.persist_count_tokens_allowed(false).await;
                        if allow_fallback {
                            return Ok(Self::local_count_tokens_response(&p));
                        }
                    }
                    let is_last_attempt = idx + 1 == attempts.len();
                    let should_retry = use_1m
                        && is_sonnet
                        && !is_last_attempt
                        && Self::is_context_1m_forbidden(&err);

                    if should_retry {
                        warn!(
                            "1M context not available for current cookie, disabling automatic 1M attempts"
                        );
                        self.persist_claude_1m_support(false).await;
                        last_err = Some(err);
                        continue;
                    }
                    return Err(err);
                }
            }
        }

        Err(last_err.unwrap_or_else(|| ClewdrError::TooManyRetries))
    }

    async fn handle_success_response(
        &mut self,
        response: wreq::Response,
        mark_support_true: bool,
        model_family: ModelFamily,
    ) -> Result<axum::response::Response, ClewdrError> {
        if !self.stream {
            let (resp, usage_pair) = Self::materialize_non_stream_response(response).await?;
            let (input, output) = usage_pair.unwrap_or((self.usage.input_tokens as u64, 0));
            self.persist_usage_totals(input, output, model_family).await;
            if mark_support_true {
                self.persist_claude_1m_support(true).await;
            }
            Ok(resp)
        } else {
            if mark_support_true {
                self.persist_claude_1m_support(true).await;
            }
            // Stream pass-through while accumulating output token usage from message_delta events
            return self.forward_stream_with_usage(response, model_family).await;
        }
    }

    async fn persist_usage_totals(&mut self, input: u64, output: u64, family: ModelFamily) {
        if input == 0 && output == 0 {
            return;
        }
        if let Some(cookie) = self.cookie.as_mut() {
            // Lazy boundary refresh if due, then reset period counters and start fresh
            Self::update_cookie_boundaries_if_due(cookie).await;
            cookie.add_and_bucket_usage(input, output, family);
            let cloned = cookie.clone();
            if let Err(err) = self.cookie_actor_handle.return_cookie(cloned, None).await {
                warn!("Failed to persist usage statistics: {}", err);
            }
        }
    }

    async fn forward_stream_with_usage(
        &mut self,
        response: wreq::Response,
        family: ModelFamily,
    ) -> Result<axum::response::Response, ClewdrError> {
        use std::sync::{
            Arc,
            atomic::{AtomicU64, Ordering},
        };

        let input_tokens = self.usage.input_tokens as u64;
        let output_sum = Arc::new(AtomicU64::new(0));
        let handle = self.cookie_actor_handle.clone();
        let cookie = self.cookie.clone();

        let osum = output_sum.clone();
        let stream = response.bytes_stream().eventsource().map_ok(move |event| {
            // accumulate output tokens from message_delta usage if present
            if let Ok(parsed) =
                serde_json::from_str::<crate::types::claude::StreamEvent>(&event.data)
            {
                match parsed {
                    crate::types::claude::StreamEvent::MessageDelta { usage: Some(u), .. } => {
                        osum.fetch_add(u.output_tokens as u64, Ordering::Relaxed);
                    }
                    crate::types::claude::StreamEvent::MessageStop => {
                        // on stream completion, persist totals asynchronously
                        if let (Some(cookie), handle) = (cookie.clone(), handle.clone()) {
                            let total_out = osum.load(Ordering::Relaxed);
                            let mut c = cookie.clone();
                            tokio::spawn(async move {
                                // Update period boundaries if needed, then accumulate
                                ClaudeCodeState::update_cookie_boundaries_if_due(&mut c).await;
                                c.add_and_bucket_usage(input_tokens, total_out, family);
                                let _ = handle.return_cookie(c, None).await;
                            });
                        }
                    }
                    _ => {}
                }
            }
            // mirror upstream SSE event unchanged
            let e = SseEvent::default().event(event.event).id(event.id);
            let e = if let Some(retry) = event.retry {
                e.retry(retry)
            } else {
                e
            };
            e.data(event.data)
        });

        Ok(Sse::new(stream)
            .keep_alive(Default::default())
            .into_response())
    }

    async fn materialize_non_stream_response(
        response: wreq::Response,
    ) -> Result<(axum::response::Response, Option<(u64, u64)>), ClewdrError> {
        let status = response.status();
        let headers = response.headers().clone();
        let bytes = response.bytes().await.context(WreqSnafu {
            msg: "Failed to read Claude response body",
        })?;
        let usage = Self::extract_usage_from_bytes(&bytes);

        let mut builder = http::Response::builder().status(status);
        for (key, value) in headers.iter() {
            builder = builder.header(key, value);
        }
        let response =
            builder
                .body(axum::body::Body::from(bytes))
                .map_err(|e| ClewdrError::HttpError {
                    loc: snafu::Location::generate(),
                    source: e,
                })?;
        Ok((response, usage))
    }

    fn extract_usage_from_bytes(bytes: &[u8]) -> Option<(u64, u64)> {
        // Prefer explicit usage if present
        if let Ok(value) = serde_json::from_slice::<serde_json::Value>(bytes)
            && let Some(usage) = value.get("usage")
        {
            let input = usage
                .get("input_tokens")
                .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|n| n.max(0) as u64)));
            let output = usage
                .get("output_tokens")
                .and_then(|v| v.as_u64().or_else(|| v.as_i64().map(|n| n.max(0) as u64)));
            if let (Some(i), Some(o)) = (input, output) {
                return Some((i, o));
            }
        }

        // Fallback: estimate output tokens from the Claude response content
        if let Ok(parsed) =
            serde_json::from_slice::<crate::types::claude::CreateMessageResponse>(bytes)
        {
            let output_tokens = parsed.count_tokens() as u64;
            // Input tokens already computed earlier and present in self.usage; only estimate output here
            return Some((0, output_tokens));
        }
        None
    }

    async fn execute_claude_count_tokens_request(
        &mut self,
        access_token: &str,
        body: &CreateMessageParams,
        use_context_1m: bool,
    ) -> Result<wreq::Response, ClewdrError> {
        let beta_header = if use_context_1m {
            CLAUDE_BETA_CONTEXT_1M
        } else {
            CLAUDE_BETA_BASE
        };

        self.client
            .post(format!("{}/v1/messages/count_tokens", self.endpoint))
            .bearer_auth(access_token)
            .header("anthropic-beta", beta_header)
            .header("anthropic-version", "2023-06-01")
            .json(body)
            .send()
            .await
            .context(WreqSnafu {
                msg: "Failed to call Claude count_tokens",
            })?
            .check_claude()
            .await
    }

    fn is_sonnet4_model(model: &str) -> bool {
        // Simplify detection: treat any model id containing
        // "claude-sonnet-4" as Sonnet 4.x for 1M probing.
        let m = model.to_ascii_lowercase();
        m.contains("claude-sonnet-4")
    }

    fn classify_model(model: &str) -> ModelFamily {
        let m = model.to_ascii_lowercase();
        if m.contains("opus") {
            ModelFamily::Opus
        } else if m.contains("sonnet") {
            ModelFamily::Sonnet
        } else {
            ModelFamily::Other
        }
    }

    // ---------------------------------------------
    // Lazy boundary refresh (no timers, fetch-on-due)
    // ---------------------------------------------
    async fn update_cookie_boundaries_if_due(cookie: &mut crate::config::CookieStatus) {
        let now = chrono::Utc::now().timestamp();
        const SESSION_WINDOW_SECS: i64 = 5 * 60 * 60; // 5h
        const WEEKLY_WINDOW_SECS: i64 = 7 * 24 * 60 * 60; // 7d

        let tracked = |flag: Option<bool>| flag == Some(true);
        let unknown = |flag: Option<bool>| flag.is_none();
        let due = |ts: Option<i64>| ts.map(|t| now >= t).unwrap_or(false);

        let session_tracked = tracked(cookie.session_has_reset);
        let weekly_tracked = tracked(cookie.weekly_has_reset);
        let opus_tracked = tracked(cookie.weekly_opus_has_reset);

        let session_due = session_tracked && due(cookie.session_resets_at);
        let weekly_due = weekly_tracked && due(cookie.weekly_resets_at);
        let opus_due = opus_tracked && due(cookie.weekly_opus_resets_at);

        let need_probe_unknown = unknown(cookie.session_has_reset)
            || unknown(cookie.weekly_has_reset)
            || unknown(cookie.weekly_opus_has_reset);
        let any_due = session_due || weekly_due || opus_due;

        if !(need_probe_unknown || any_due) {
            return;
        }

        cookie.resets_last_checked_at = Some(now);
        if let Some((sess, week, opus)) = Self::fetch_usage_resets(&cookie.cookie).await {
            // Unknown -> decide track/not-track
            if unknown(cookie.session_has_reset) {
                cookie.session_has_reset = Some(sess.is_some());
            }
            if unknown(cookie.weekly_has_reset) {
                cookie.weekly_has_reset = Some(week.is_some());
            }
            if unknown(cookie.weekly_opus_has_reset) {
                cookie.weekly_opus_has_reset = Some(opus.is_some());
            }

            // Handle due tracked windows: reset usage then update boundaries if provided
            if session_due {
                cookie.session_usage = crate::config::UsageBreakdown::default();
            }
            if weekly_due {
                cookie.weekly_usage = crate::config::UsageBreakdown::default();
            }
            if opus_due {
                cookie.weekly_opus_usage = crate::config::UsageBreakdown::default();
            }

            // Update/reset boundaries for tracked windows
            if cookie.session_has_reset == Some(true) {
                if let Some(ts) = sess {
                    cookie.session_resets_at = Some(ts);
                } else {
                    // Server indicates no boundary -> stop tracking and clear ts
                    cookie.session_has_reset = Some(false);
                    cookie.session_resets_at = None;
                }
            }
            if cookie.weekly_has_reset == Some(true) {
                if let Some(ts) = week {
                    cookie.weekly_resets_at = Some(ts);
                } else {
                    cookie.weekly_has_reset = Some(false);
                    cookie.weekly_resets_at = None;
                }
            }
            if cookie.weekly_opus_has_reset == Some(true) {
                if let Some(ts) = opus {
                    cookie.weekly_opus_resets_at = Some(ts);
                } else {
                    cookie.weekly_opus_has_reset = Some(false);
                    cookie.weekly_opus_resets_at = None;
                }
            }
        } else {
            // Network/parse failure: apply fallback only for windows we currently track
            if session_due && session_tracked {
                cookie.session_usage = crate::config::UsageBreakdown::default();
                cookie.session_resets_at = Some(now + SESSION_WINDOW_SECS);
            }
            if weekly_due && weekly_tracked {
                cookie.weekly_usage = crate::config::UsageBreakdown::default();
                cookie.weekly_resets_at = Some(now + WEEKLY_WINDOW_SECS);
            }
            if opus_due && opus_tracked {
                cookie.weekly_opus_usage = crate::config::UsageBreakdown::default();
                cookie.weekly_opus_resets_at = Some(now + WEEKLY_WINDOW_SECS);
            }
        }
    }

    async fn fetch_usage_resets(
        cookie: &crate::config::ClewdrCookie,
    ) -> Option<(Option<i64>, Option<i64>, Option<i64>)> {
        // Build client
        let client = crate::net::client::chrome_client_with_proxy(
            CLEWDR_CONFIG.load().wreq_proxy.clone(),
        )
        .ok()?;

        // Attach cookie for both api and console domains
        let endpoint: Url = CLEWDR_CONFIG.load().endpoint();
        let cookie_header = http::HeaderValue::from_str(&cookie.to_string()).ok()?;
        let console_url = Url::parse(CLAUDE_CONSOLE_ENDPOINT).ok()?;
        crate::net::client::attach_cookie_for_api_and_console(
            &client,
            &endpoint,
            &console_url,
            &cookie_header,
        );

        // Discover organization UUID
        let org_uuid = crate::anthropic::org::select_chat_org_uuid(&client, &endpoint).await?;
        // Fetch usage JSON and parse timestamps
        let usage = crate::anthropic::usage::fetch_console_usage(&client, &org_uuid).await?;
        Some(crate::anthropic::usage::parse_reset_timestamps(&usage))
    }

    fn local_count_tokens_response(body: &CreateMessageParams) -> axum::response::Response {
        let estimate = CountMessageTokensResponse {
            input_tokens: body.count_tokens(),
        };
        Json(estimate).into_response()
    }

    fn is_context_1m_forbidden(error: &ClewdrError) -> bool {
        if let ClewdrError::ClaudeHttpError { code, inner } = error
            && (code.as_u16() == 403 || code.as_u16() == 400)
        {
            let message = inner
                .message
                .as_str()
                .map(|s| s.to_ascii_lowercase())
                .unwrap_or_default();
            // Different account tiers (e.g., Pro vs Max) surface different error texts
            // when 1M context is not permitted. Treat both as a signal to fallback.
            return message.contains(
                "the long context beta is not yet available for this subscription.",
            ) || message.contains(
                "this authentication style is incompatible with the long context beta header.",
            );
        }
        false
    }

    fn is_count_tokens_unauthorized(error: &ClewdrError) -> bool {
        if let ClewdrError::ClaudeHttpError { code, .. } = error {
            return match code.as_u16() {
                401 | 404 => true,
                403 => !Self::is_context_1m_forbidden(error),
                _ => false,
            };
        }
        false
    }
}
