use std::{collections::HashMap, pin::Pin, str::FromStr};

use oauth2::{
    AsyncHttpClient, AuthorizationCode, Client, ClientId, CsrfToken, EndpointNotSet, EndpointSet,
    HttpClientError, HttpRequest, HttpResponse, PkceCodeChallenge, PkceCodeVerifier, RedirectUrl,
    Scope, StandardRevocableToken, TokenUrl,
    basic::{
        BasicErrorResponse, BasicRevocationErrorResponse, BasicTokenIntrospectionResponse,
        BasicTokenResponse,
    },
    http,
};
use snafu::{ResultExt, OptionExt};
use url::Url;

use crate::{
    config::{CC_REDIRECT_URI, CC_TOKEN_URL, TokenInfo},
    error::{ClewdrError, UnexpectedNoneSnafu, UrlSnafu, WreqSnafu},
};

type ClaudeOauthClient = Client<
    BasicErrorResponse,
    BasicTokenResponse,
    BasicTokenIntrospectionResponse,
    StandardRevocableToken,
    BasicRevocationErrorResponse,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointNotSet,
    EndpointSet,
>;

struct OauthClient {
    client: wreq::Client,
}

impl<'c> AsyncHttpClient<'c> for OauthClient {
    type Error = HttpClientError<wreq::Error>;

    type Future = Pin<Box<dyn std::future::Future<Output = Result<HttpResponse, Self::Error>> + Send + Sync + 'c>>;

    fn call(&'c self, request: HttpRequest) -> Self::Future {
        Box::pin(async move {
            let response = self
                .client
                .execute(request.try_into().map_err(Box::new)?)
                .await
                .map_err(Box::new)?;

            let mut builder = http::Response::builder().status(response.status());
            builder = builder.version(response.version());
            for (name, value) in response.headers().iter() {
                builder = builder.header(name, value);
            }
            builder
                .body(response.bytes().await.map_err(Box::new)?.to_vec())
                .map_err(HttpClientError::Http)
        })
    }
}

fn setup_client(cc_client_id: String) -> Result<ClaudeOauthClient, ClewdrError> {
    Ok(oauth2::basic::BasicClient::new(ClientId::new(cc_client_id))
        .set_auth_type(oauth2::AuthType::RequestBody)
        .set_redirect_uri(RedirectUrl::new(CC_REDIRECT_URI.into()).map_err(|_| {
            ClewdrError::UnexpectedNone { msg: "Invalid redirect URI" }
        })?)
        .set_token_uri(TokenUrl::new(CC_TOKEN_URL.into()).map_err(|_| {
            ClewdrError::UnexpectedNone { msg: "Invalid token URI" }
        })?))
}

#[derive(Debug)]
pub struct ExchangeCodeResult {
    pub code: String,
    pub state: Option<String>,
    pub verifier: PkceCodeVerifier,
}

/// Perform PKCE auth to obtain authorization code for an organization
pub async fn exchange_code(
    client: &wreq::Client,
    endpoint: &Url,
    org_uuid: &str,
    cc_client_id: String,
) -> Result<ExchangeCodeResult, ClewdrError> {
    let authorize_url = |org_uuid: &str| {
        format!("{}/v1/oauth/{}/authorize", endpoint, org_uuid)
    };
    let oauth_client_cfg = setup_client(cc_client_id)?.set_auth_uri(
        oauth2::AuthUrl::new(authorize_url(org_uuid)).map_err(|_| ClewdrError::UnexpectedNone {
            msg: "Invalid auth URI",
        })?,
    );

    let (pkce_challenge, pkce_verifier) = PkceCodeChallenge::new_random_sha256();
    let (mut auth_url, _csrf_token) = oauth_client_cfg
        .authorize_url(|| CsrfToken::new_random_len(32))
        .add_scope(Scope::new("user:profile".to_string()))
        .add_scope(Scope::new("user:inference".to_string()))
        .set_pkce_challenge(pkce_challenge)
        .url();

    let mut query_params: HashMap<String, String> =
        auth_url.query_pairs().into_owned().collect();
    query_params.insert("organization_uuid".to_string(), org_uuid.to_string());
    auth_url.set_query(None);

    let redirect_json = client
        .post(auth_url)
        .json(&query_params)
        .send()
        .await
        .context(WreqSnafu { msg: "Failed to send authorization request" })?
        .json::<serde_json::Value>()
        .await
        .context(WreqSnafu { msg: "Failed to parse authorization response" })?;

    let redirect_uri = redirect_json["redirect_uri"]
        .as_str()
        .ok_or(ClewdrError::UnexpectedNone { msg: "redirect_uri missing" })?;
    let redirect_url = Url::from_str(redirect_uri).context(UrlSnafu { url: redirect_uri.to_string() })?;
    let query = redirect_url.query_pairs().collect::<HashMap<_, _>>();
    let code = query.get("code").context(UnexpectedNoneSnafu { msg: "No code found in redirect URL" })?;
    let state = query.get("state");

    Ok(ExchangeCodeResult {
        code: code.to_string(),
        state: state.map(|s| s.to_string()),
        verifier: pkce_verifier,
    })
}

/// Exchange authorization code for access/refresh tokens
pub async fn exchange_token(
    client: &wreq::Client,
    cc_client_id: String,
    code_res: ExchangeCodeResult,
    org_uuid: String,
) -> Result<TokenInfo, ClewdrError> {
    let oauth_client_cfg = setup_client(cc_client_id)?;
    let my_client = OauthClient { client: client.clone() };

    let mut token_request = oauth_client_cfg
        .exchange_code(AuthorizationCode::new(code_res.code))
        .set_pkce_verifier(code_res.verifier);
    if let Some(state) = code_res.state { token_request = token_request.add_extra_param("state", state); }

    let token = token_request.request_async(&my_client).await?;
    Ok(TokenInfo::new(token, org_uuid))
}

/// Refresh access token using refresh_token
pub async fn refresh_token(
    client: &wreq::Client,
    cc_client_id: String,
    refresh_token: String,
    org_uuid: String,
) -> Result<TokenInfo, ClewdrError> {
    let client_cfg = oauth2::basic::BasicClient::new(ClientId::new(cc_client_id))
        .set_auth_type(oauth2::AuthType::RequestBody)
        .set_token_uri(TokenUrl::new(CC_TOKEN_URL.into()).map_err(|_| ClewdrError::UnexpectedNone { msg: "Invalid token URI" })?);

    let my_client = OauthClient { client: client.clone() };
    let new_token = client_cfg
        .exchange_refresh_token(&oauth2::RefreshToken::new(refresh_token))
        .request_async(&my_client)
        .await?;
    Ok(TokenInfo::new(new_token, org_uuid))
}
