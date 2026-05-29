//! Google OAuth 2.0 helpers for the Gmail flow.
//!
//! The desktop / mobile shell drives the dance:
//! 1. [`bind_oauth_listener`] picks a free localhost port.
//! 2. [`build_auth_url`] builds the Google authorize URL with that port as the
//!    redirect URI; the shell opens it in the system browser.
//! 3. [`wait_for_oauth_code`] reads exactly one HTTP request off the listener,
//!    extracts the `code` query param, and writes a small success page back.
//! 4. [`exchange_code_for_tokens`] swaps the code for a refresh token plus an
//!    initial access token.
//! 5. [`ensure_access_token`] keeps the access token fresh between Gmail
//!    requests.

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

use crate::{Error, Result};

const AUTH_URL: &str = "https://accounts.google.com/o/oauth2/v2/auth";
const TOKEN_URL: &str = "https://oauth2.googleapis.com/token";
pub const GMAIL_READONLY_SCOPE: &str = "https://www.googleapis.com/auth/gmail.readonly";

const PAYPAL_AUTH_URL: &str = "https://www.paypal.com/signin/authorize";
const PAYPAL_TOKEN_URL: &str = "https://api-m.paypal.com/v1/oauth2/token";
pub const PAYPAL_OPENID_SCOPE: &str = "openid";
pub const PAYPAL_TRANSACTIONS_SCOPE: &str =
    "https://uri.paypal.com/services/reporting/search/read";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct OAuthCredentials {
    pub client_id: String,
    pub client_secret: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tokens {
    pub refresh_token: String,
    pub access_token: Option<String>,
    pub expires_at: Option<DateTime<Utc>>,
}

/// Bind a TCP listener on a free localhost port; return the listener plus the
/// port number the OS assigned.
pub async fn bind_oauth_listener() -> Result<(TcpListener, u16)> {
    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|e| Error::Config(format!("failed to bind oauth callback port: {e}")))?;
    let port = listener
        .local_addr()
        .map_err(|e| Error::Config(format!("failed to read local addr: {e}")))?
        .port();
    Ok((listener, port))
}

fn percent_encode(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

#[must_use]
pub fn build_auth_url(client_id: &str, redirect_uri: &str, scopes: &[&str]) -> String {
    let scope = scopes.join(" ");
    format!(
        "{AUTH_URL}?response_type=code&client_id={cid}&redirect_uri={ru}&scope={sc}&access_type=offline&prompt=consent",
        cid = percent_encode(client_id),
        ru = percent_encode(redirect_uri),
        sc = percent_encode(&scope),
    )
}

/// Wait for exactly one HTTP request on the listener, extract the `code` query
/// parameter, write back a small success page, return the code.
pub async fn wait_for_oauth_code(listener: TcpListener) -> Result<String> {
    let (mut socket, _) = listener
        .accept()
        .await
        .map_err(|e| Error::Config(format!("oauth callback accept failed: {e}")))?;

    let mut buf = vec![0u8; 8192];
    let n = socket
        .read(&mut buf)
        .await
        .map_err(|e| Error::Config(format!("oauth callback read failed: {e}")))?;
    let req = String::from_utf8_lossy(&buf[..n]);

    let path = req
        .lines()
        .next()
        .and_then(|line| line.split(' ').nth(1))
        .ok_or_else(|| Error::Config("oauth callback malformed HTTP".into()))?;

    let code = extract_query_param(path, "code")
        .ok_or_else(|| Error::Config("oauth callback missing code".into()))?;

    let body = "<!doctype html><html><head><title>kinketsu</title><meta charset=\"utf-8\"></head><body style=\"font-family:system-ui,sans-serif;padding:3rem;text-align:center;background:#0e0e1a;color:#fafaff\"><h1 style=\"font-weight:800;letter-spacing:-0.02em\">kinketsu</h1><p style=\"opacity:0.7\">Gmail connected. You can close this tab.</p></body></html>";
    let response = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        body.len(),
        body
    );
    let _ = socket.write_all(response.as_bytes()).await;
    let _ = socket.shutdown().await;

    Ok(code)
}

fn extract_query_param(path: &str, key: &str) -> Option<String> {
    let (_, query) = path.split_once('?')?;
    for pair in query.split('&') {
        let (k, v) = pair.split_once('=')?;
        if k == key {
            return Some(
                url::form_urlencoded::parse(v.as_bytes())
                    .next()
                    .map(|(_, v)| v.into_owned())
                    .unwrap_or_else(|| v.to_string()),
            );
        }
    }
    None
}

pub async fn exchange_code_for_tokens(
    creds: &OAuthCredentials,
    code: &str,
    redirect_uri: &str,
) -> Result<Tokens> {
    let resp = reqwest::Client::new()
        .post(TOKEN_URL)
        .form(&[
            ("code", code),
            ("client_id", creds.client_id.as_str()),
            ("client_secret", creds.client_secret.as_str()),
            ("redirect_uri", redirect_uri),
            ("grant_type", "authorization_code"),
        ])
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(Error::Config(format!("token exchange {status}: {text}")));
    }
    let body: serde_json::Value = resp.json().await?;
    let refresh_token = body
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| Error::Config("token response missing refresh_token".into()))?;
    let access_token = body
        .get("access_token")
        .and_then(|v| v.as_str())
        .map(String::from);
    let expires_at = body
        .get("expires_in")
        .and_then(|v| v.as_i64())
        .map(|s| Utc::now() + Duration::seconds(s));
    Ok(Tokens {
        refresh_token,
        access_token,
        expires_at,
    })
}

pub async fn refresh_access_token(
    creds: &OAuthCredentials,
    refresh_token: &str,
) -> Result<(String, DateTime<Utc>)> {
    let resp = reqwest::Client::new()
        .post(TOKEN_URL)
        .form(&[
            ("client_id", creds.client_id.as_str()),
            ("client_secret", creds.client_secret.as_str()),
            ("refresh_token", refresh_token),
            ("grant_type", "refresh_token"),
        ])
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(Error::Config(format!("token refresh {status}: {text}")));
    }
    let body: serde_json::Value = resp.json().await?;
    let access_token = body
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Config("refresh response missing access_token".into()))?
        .to_string();
    let expires_in = body
        .get("expires_in")
        .and_then(|v| v.as_i64())
        .unwrap_or(3600);
    Ok((access_token, Utc::now() + Duration::seconds(expires_in)))
}

// ---- PayPal (Log In with PayPal) ----

#[must_use]
pub fn build_paypal_auth_url(client_id: &str, redirect_uri: &str, scopes: &[&str]) -> String {
    let scope = scopes.join(" ");
    format!(
        "{PAYPAL_AUTH_URL}?response_type=code&client_id={cid}&redirect_uri={ru}&scope={sc}",
        cid = percent_encode(client_id),
        ru = percent_encode(redirect_uri),
        sc = percent_encode(&scope),
    )
}

pub async fn exchange_paypal_code(
    creds: &OAuthCredentials,
    code: &str,
    redirect_uri: &str,
) -> Result<Tokens> {
    let resp = reqwest::Client::new()
        .post(PAYPAL_TOKEN_URL)
        .basic_auth(&creds.client_id, Some(&creds.client_secret))
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code),
            ("redirect_uri", redirect_uri),
        ])
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(Error::Config(format!(
            "paypal token exchange {status}: {text}"
        )));
    }
    let body: serde_json::Value = resp.json().await?;
    let refresh_token = body
        .get("refresh_token")
        .and_then(|v| v.as_str())
        .map(String::from)
        .ok_or_else(|| Error::Config("paypal token response missing refresh_token".into()))?;
    let access_token = body
        .get("access_token")
        .and_then(|v| v.as_str())
        .map(String::from);
    let expires_at = body
        .get("expires_in")
        .and_then(|v| v.as_i64())
        .map(|s| Utc::now() + Duration::seconds(s));
    Ok(Tokens {
        refresh_token,
        access_token,
        expires_at,
    })
}

pub async fn refresh_paypal_access_token(
    creds: &OAuthCredentials,
    refresh_token: &str,
) -> Result<(String, DateTime<Utc>)> {
    let resp = reqwest::Client::new()
        .post(PAYPAL_TOKEN_URL)
        .basic_auth(&creds.client_id, Some(&creds.client_secret))
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token),
        ])
        .send()
        .await?;
    let status = resp.status();
    if !status.is_success() {
        let text = resp.text().await.unwrap_or_default();
        return Err(Error::Config(format!(
            "paypal token refresh {status}: {text}"
        )));
    }
    let body: serde_json::Value = resp.json().await?;
    let access_token = body
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| Error::Config("paypal refresh response missing access_token".into()))?
        .to_string();
    let expires_in = body
        .get("expires_in")
        .and_then(|v| v.as_i64())
        .unwrap_or(3600);
    Ok((access_token, Utc::now() + Duration::seconds(expires_in)))
}

pub async fn ensure_paypal_access_token(
    creds: &OAuthCredentials,
    tokens: &mut Tokens,
) -> Result<String> {
    let now = Utc::now();
    if let (Some(tok), Some(exp)) = (tokens.access_token.as_ref(), tokens.expires_at) {
        if exp > now + Duration::seconds(60) {
            return Ok(tok.clone());
        }
    }
    let (new_token, new_expires) =
        refresh_paypal_access_token(creds, &tokens.refresh_token).await?;
    tokens.access_token = Some(new_token.clone());
    tokens.expires_at = Some(new_expires);
    Ok(new_token)
}

// ---- Google (Gmail) ----

/// Return a fresh access token, refreshing in-place if the cached one is
/// expired (or within 60 seconds of expiry).
pub async fn ensure_access_token(
    creds: &OAuthCredentials,
    tokens: &mut Tokens,
) -> Result<String> {
    let now = Utc::now();
    if let (Some(tok), Some(exp)) = (tokens.access_token.as_ref(), tokens.expires_at) {
        if exp > now + Duration::seconds(60) {
            return Ok(tok.clone());
        }
    }
    let (new_token, new_expires) = refresh_access_token(creds, &tokens.refresh_token).await?;
    tokens.access_token = Some(new_token.clone());
    tokens.expires_at = Some(new_expires);
    Ok(new_token)
}
