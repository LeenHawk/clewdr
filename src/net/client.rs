use snafu::ResultExt;
use url::Url;
use wreq::{Client, ClientBuilder};
use wreq_util::Emulation;

use crate::{
    error::{ClewdrError, WreqSnafu},
};

/// Build a Chrome-like client with optional upstream proxy and cookie store enabled.
pub fn chrome_client_with_proxy(proxy: Option<wreq::Proxy>) -> Result<Client, ClewdrError> {
    let mut builder = ClientBuilder::new()
        .cookie_store(true)
        .emulation(Emulation::Chrome136);
    if let Some(proxy) = proxy {
        builder = builder.proxy(proxy);
    }
    builder.build().context(WreqSnafu { msg: "Failed to build client" })
}

/// Attach a cookie header value to both API endpoint domain and console domain.
pub fn attach_cookie_for_api_and_console(
    client: &Client,
    api_endpoint: &Url,
    console_endpoint: &Url,
    cookie_header: &http::HeaderValue,
) {
    client.set_cookie(api_endpoint, cookie_header);
    client.set_cookie(console_endpoint, cookie_header);
}

