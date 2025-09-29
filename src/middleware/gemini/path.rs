use axum::extract::{FromRequestParts, Query};
use axum_auth::AuthBearer;
use serde::Deserialize;
use struct_iterable::Iterable;

use crate::error::ClewdrError;

#[derive(Debug, Clone, Deserialize, Iterable, Default)]
pub struct GeminiArgs {
    pub key: String,
    pub alt: Option<String>,
}

#[derive(Deserialize)]
struct GeminiQueryAlt {
    pub alt: Option<String>,
}

impl<S> FromRequestParts<S> for GeminiArgs
where
    S: Sync,
{
    type Rejection = ClewdrError;

    async fn from_request_parts(
        parts: &mut axum::http::request::Parts,
        _: &S,
    ) -> Result<Self, Self::Rejection> {
        match Query::<Self>::from_request_parts(parts, &()).await {
            Ok(Query(q)) => Ok(q),
            Err(_) => {
                let Query(q) = Query::<GeminiQueryAlt>::from_request_parts(parts, &()).await?;
                // Prefer x-goog-api-key. If absent and this is gemini-cli route,
                // accept Authorization: Bearer as a flexible auth option.
                // x-goog-api-key first
                let key = parts
                    .headers
                    .get("x-goog-api-key")
                    .and_then(|v| v.to_str().ok())
                    .map(|s| s.to_string());
                let key = if let Some(k) = key {
                    k
                } else if parts.uri.path().contains("/gemini-cli/") {
                    // Fallback to Authorization: Bearer for gemini-cli routes
                    match AuthBearer::from_request_parts(parts, &()).await {
                        Ok(AuthBearer(token)) => token,
                        Err(_) => return Err(ClewdrError::InvalidAuth),
                    }
                } else {
                    return Err(ClewdrError::InvalidAuth);
                };
                Ok(Self {
                    key,
                    alt: q.alt,
                })
            }
        }
    }
}

impl GeminiArgs {
    pub fn to_vec(&self) -> Vec<(&'static str, &str)> {
        let mut vec = Vec::new();
        for (k, vv) in self.iter() {
            if k == "key" {
                continue;
            }
            if let Some(v) = vv
                .downcast_ref::<String>()
                .or(vv.downcast_ref::<Option<String>>().and_then(|v| v.as_ref()))
            {
                vec.push((k, v.as_str()));
            }
        }
        vec
    }
}
