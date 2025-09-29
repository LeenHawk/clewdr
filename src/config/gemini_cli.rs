use std::collections::HashSet;

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Default endpoint used by the Gemini CLI (Code Assist) surface.
pub const DEFAULT_CODE_ASSIST_ENDPOINT: &str = "https://cloudcode-pa.googleapis.com";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
pub struct GeminiCliCredential {
    pub client_id: String,
    pub client_secret: String,
    pub token: String,
    pub refresh_token: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    pub token_uri: String,
    pub project_id: String,
    #[serde(default, with = "option_iso8601")]
    pub expiry: Option<DateTime<Utc>>,
}

impl GeminiCliCredential {
    pub fn identifier(&self) -> String {
        format!("{}::{}", self.client_id, self.project_id)
    }

    pub fn needs_refresh(&self) -> bool {
        match self.expiry {
            Some(expiry) => expiry <= Utc::now() + Duration::seconds(60),
            None => self.token.trim().is_empty(),
        }
    }

    pub fn with_token(
        &self,
        token: String,
        expiry: Option<DateTime<Utc>>,
        scopes: Option<Vec<String>>,
    ) -> Self {
        let mut updated = self.clone();
        updated.token = token;
        if let Some(expiry) = expiry {
            updated.expiry = Some(expiry);
        }
        if let Some(scopes) = scopes.and_then(|s| (!s.is_empty()).then_some(s)) {
            updated.scopes = scopes;
        }
        updated
    }

    pub fn is_valid(&self) -> bool {
        !self.client_id.trim().is_empty()
            && !self.client_secret.trim().is_empty()
            && !self.refresh_token.trim().is_empty()
            && !self.token_uri.trim().is_empty()
            && !self.project_id.trim().is_empty()
    }
}

pub fn dedup_cli_credentials(mut creds: Vec<GeminiCliCredential>) -> Vec<GeminiCliCredential> {
    let mut seen = HashSet::new();
    creds.retain(|cred| {
        if !cred.is_valid() {
            return false;
        }
        seen.insert(cred.identifier())
    });
    creds
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeminiCliCredentialSummary {
    pub client_id: String,
    pub project_id: String,
    #[serde(default)]
    pub scopes: Vec<String>,
    #[serde(default, with = "option_iso8601")]
    pub expiry: Option<DateTime<Utc>>,
    pub has_refresh_token: bool,
}

impl From<&GeminiCliCredential> for GeminiCliCredentialSummary {
    fn from(value: &GeminiCliCredential) -> Self {
        Self {
            client_id: value.client_id.clone(),
            project_id: value.project_id.clone(),
            scopes: value.scopes.clone(),
            expiry: value.expiry,
            has_refresh_token: !value.refresh_token.trim().is_empty(),
        }
    }
}

mod option_iso8601 {
    use chrono::{DateTime, Utc};
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(value: &Option<DateTime<Utc>>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match value {
            Some(dt) => serializer.serialize_some(&dt.to_rfc3339()),
            None => serializer.serialize_none(),
        }
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Option<DateTime<Utc>>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let opt = Option::<String>::deserialize(deserializer)?;
        match opt {
            Some(value) => {
                let parsed = DateTime::parse_from_rfc3339(&value)
                    .map_err(serde::de::Error::custom)?
                    .with_timezone(&Utc);
                Ok(Some(parsed))
            }
            None => Ok(None),
        }
    }
}
