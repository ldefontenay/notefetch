//! Signing in to Plaud, renewing the sign-in, and keeping it safe on disk.
//!
//! The flow, proven in milestone 1: discover the authorisation server, listen
//! on a free loopback port, register a new public client for exactly that
//! address, open the browser at Plaud's sign-in page with PKCE, wait for the
//! redirect, and exchange the code for tokens. Registering afresh at every
//! sign-in means the port never has to match an earlier registration.
//!
//! The tokens and client ID are kept in `auth.bin`, encrypted with DPAPI for
//! the current Windows user. Nothing here is ever logged or printed.
//!
//! Adapted from plaud-cli (https://github.com/leegonzales/plaud-cli), used
//! under the MIT licence:
//!
//!   Copyright (c) 2026 Lee Gonzales
//!
//!   Permission is hereby granted, free of charge, to any person obtaining a
//!   copy of this software and associated documentation files (the
//!   "Software"), to deal in the Software without restriction, including
//!   without limitation the rights to use, copy, modify, merge, publish,
//!   distribute, sublicense, and/or sell copies of the Software, and to permit
//!   persons to whom the Software is furnished to do so, subject to the
//!   following conditions:
//!
//!   The above copyright notice and this permission notice shall be included
//!   in all copies or substantial portions of the Software.
//!
//!   THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS
//!   OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF
//!   MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN
//!   NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM,
//!   DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR
//!   OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE
//!   USE OR OTHER DEALINGS IN THE SOFTWARE.

use std::path::PathBuf;
use std::time::Duration;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use url::Url;
use windows::Win32::Foundation::{HLOCAL, LocalFree};
use windows::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};

use crate::cancel::Cancel;
use crate::clock;
use crate::error::{Error, Result};

pub const MCP_URL: &str = "https://mcp.plaud.ai/mcp";
const PROTECTED_RESOURCE_URL: &str = "https://mcp.plaud.ai/.well-known/oauth-protected-resource/mcp";
const CLIENT_NAME: &str = "NoteFetch";
const SIGN_IN_TIMEOUT: Duration = Duration::from_secs(300);
const AUTH_FILE: &str = "auth.bin";

/// Everything needed to call Plaud as the signed-in user. Never logged.
#[derive(Clone, Serialize, Deserialize)]
pub struct Tokens {
    pub client_id: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    /// Unix seconds.
    pub expires_at: Option<i64>,
}

// Deliberately no Debug derive: a stray {:?} must not be able to print a token.
impl std::fmt::Debug for Tokens {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Tokens(hidden)")
    }
}

fn auth_path() -> PathBuf {
    crate::config::roaming_dir().join(AUTH_FILE)
}

/// The stored sign-in, if any.
pub fn load() -> Result<Option<Tokens>> {
    let path = auth_path();
    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(crate::error::disk("read your sign-in from", &path, &e)),
    };
    let Some(plain) = unprotect(&bytes) else {
        // Encrypted for another Windows user, or damaged: sign in again.
        return Ok(None);
    };
    Ok(serde_json::from_slice(&plain).ok())
}

pub fn save(tokens: &Tokens) -> Result<()> {
    let path = auth_path();
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| crate::error::disk("create the folder", dir, &e))?;
    }
    let plain = serde_json::to_vec(tokens).map_err(|e| Error::Disk(format!("Could not prepare your sign-in for saving: {e}.")))?;
    let secret = protect(&plain).ok_or_else(|| Error::Disk("Windows could not encrypt your sign-in, so it was not saved.".into()))?;
    crate::config::write_atomically(&path, &secret).map_err(|e| crate::error::disk("save your sign-in to", &path, &e))
}

/// Sign out: forget the tokens and the client registration.
pub fn forget() -> Result<()> {
    let path = auth_path();
    match std::fs::remove_file(&path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(crate::error::disk("remove your sign-in from", &path, &e)),
    }
}

/// Encrypt with DPAPI for the current Windows user.
pub fn protect(plain: &[u8]) -> Option<Vec<u8>> {
    let input = CRYPT_INTEGER_BLOB { cbData: plain.len() as u32, pbData: plain.as_ptr() as *mut u8 };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // SAFETY: input points at `plain` for the length given; output is freed below.
    unsafe {
        CryptProtectData(&input, windows::core::w!("NoteFetch sign-in"), None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output).ok()?;
        Some(take_blob(output))
    }
}

pub fn unprotect(secret: &[u8]) -> Option<Vec<u8>> {
    let input = CRYPT_INTEGER_BLOB { cbData: secret.len() as u32, pbData: secret.as_ptr() as *mut u8 };
    let mut output = CRYPT_INTEGER_BLOB::default();
    // SAFETY: as for protect.
    unsafe {
        CryptUnprotectData(&input, None, None, None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut output).ok()?;
        Some(take_blob(output))
    }
}

/// Copy a blob Windows allocated, then free it.
unsafe fn take_blob(blob: CRYPT_INTEGER_BLOB) -> Vec<u8> {
    // SAFETY: Windows returned pbData valid for cbData bytes, allocated with LocalAlloc.
    unsafe {
        let bytes = std::slice::from_raw_parts(blob.pbData, blob.cbData as usize).to_vec();
        LocalFree(Some(HLOCAL(blob.pbData as *mut _)));
        bytes
    }
}

struct Endpoints {
    authorization: String,
    token: String,
    registration: Option<String>,
}

async fn discover(http: &reqwest::Client) -> Result<Endpoints> {
    let prm: Value = http.get(PROTECTED_RESOURCE_URL).send().await?.json().await?;
    let issuer = prm["authorization_servers"][0]
        .as_str()
        .ok_or_else(|| Error::Plaud("its sign-in details did not name a sign-in server.".into()))?
        .trim_end_matches('/')
        .to_string();
    let meta: Value = http.get(format!("{issuer}/.well-known/oauth-authorization-server")).send().await?.json().await?;
    let field = |k: &str| meta[k].as_str().map(str::to_string);
    Ok(Endpoints {
        authorization: field("authorization_endpoint").ok_or_else(|| Error::Plaud("its sign-in details were incomplete.".into()))?,
        token: field("token_endpoint").ok_or_else(|| Error::Plaud("its sign-in details were incomplete.".into()))?,
        registration: field("registration_endpoint"),
    })
}

fn random_url_safe(bytes: usize) -> String {
    let mut raw = vec![0u8; bytes];
    rand::fill(&mut raw[..]);
    URL_SAFE_NO_PAD.encode(raw)
}

/// Run the whole sign-in. `status` receives plain progress lines for the Log.
/// Cancelled by `cancel`; gives up after five minutes.
pub async fn sign_in(http: &reqwest::Client, cancel: &Cancel, status: &(dyn Fn(String) + Sync)) -> Result<Tokens> {
    let ep = discover(http).await?;
    let registration = ep
        .registration
        .ok_or_else(|| Error::SignIn("Plaud no longer accepts new apps in the way NoteFetch expects.".into()))?;

    let listener = TcpListener::bind("127.0.0.1:0")
        .await
        .map_err(|_| Error::SignIn("Windows would not let NoteFetch listen for the browser's reply.".into()))?;
    let port = listener
        .local_addr()
        .map_err(|_| Error::SignIn("Windows would not let NoteFetch listen for the browser's reply.".into()))?
        .port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");

    let resp = http
        .post(&registration)
        .json(&json!({
            "client_name": CLIENT_NAME,
            "redirect_uris": [redirect_uri],
            "grant_types": ["authorization_code", "refresh_token"],
            "response_types": ["code"],
            "token_endpoint_auth_method": "none",
        }))
        .send()
        .await?;
    let reg_status = resp.status().as_u16();
    let reg: Value = resp.json().await.unwrap_or(Value::Null);
    let client_id = reg["client_id"]
        .as_str()
        .ok_or_else(|| Error::SignIn(format!("Plaud refused to register NoteFetch (HTTP {reg_status}).")))?
        .to_string();

    let verifier = random_url_safe(32);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    let state = random_url_safe(16);
    let mut auth_url = Url::parse(&ep.authorization).map_err(|_| Error::Plaud("its sign-in address was not valid.".into()))?;
    auth_url
        .query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("state", &state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("resource", MCP_URL);

    open_in_browser(auth_url.as_str())?;
    status("Plaud's sign-in page is open in your web browser. Sign in there, then come back to NoteFetch.".into());

    let code = tokio::select! {
        r = wait_for_callback(listener, &state) => r?,
        _ = tokio::time::sleep(SIGN_IN_TIMEOUT) => {
            return Err(Error::SignIn("no reply came from the browser within five minutes.".into()));
        }
        _ = cancel.cancelled() => return Err(Error::Cancelled),
    };

    let resp = http
        .post(&ep.token)
        .form(&[
            ("grant_type", "authorization_code"),
            ("code", code.as_str()),
            ("redirect_uri", redirect_uri.as_str()),
            ("client_id", client_id.as_str()),
            ("code_verifier", verifier.as_str()),
            ("resource", MCP_URL),
        ])
        .send()
        .await?;
    let tokens = tokens_from_reply(resp, client_id, None).await.map_err(|e| match e {
        Error::SignInExpired => Error::SignIn("Plaud did not accept the sign-in code.".into()),
        other => other,
    })?;
    save(&tokens)?;
    Ok(tokens)
}

/// Renew the access token. Plaud returns a new refresh token every time
/// (proven in milestone 1), and the new one is saved before returning, so the
/// old one is never needed again. The caller makes sure only one renewal runs
/// at a time.
pub async fn refresh(http: &reqwest::Client, tokens: &Tokens) -> Result<Tokens> {
    let refresh_token = tokens.refresh_token.clone().ok_or(Error::SignInExpired)?;
    let ep = discover(http).await?;
    let resp = http
        .post(&ep.token)
        .form(&[
            ("grant_type", "refresh_token"),
            ("refresh_token", refresh_token.as_str()),
            ("client_id", tokens.client_id.as_str()),
            ("resource", MCP_URL),
        ])
        .send()
        .await?;
    let fresh = tokens_from_reply(resp, tokens.client_id.clone(), Some(refresh_token)).await?;
    save(&fresh)?;
    Ok(fresh)
}

async fn tokens_from_reply(resp: reqwest::Response, client_id: String, old_refresh: Option<String>) -> Result<Tokens> {
    let status = resp.status();
    let reply: Value = resp.json().await.unwrap_or(Value::Null);
    if status.is_client_error() {
        // invalid_grant and the like: the sign-in is no longer valid.
        return Err(Error::SignInExpired);
    }
    if !status.is_success() {
        return Err(Error::Plaud(format!("its sign-in server returned HTTP {}.", status.as_u16())));
    }
    let access = reply["access_token"]
        .as_str()
        .ok_or_else(|| Error::Plaud("its sign-in reply held no access token.".into()))?;
    let now = clock::now_utc().unix_millis() / 1000;
    Ok(Tokens {
        client_id,
        access_token: access.to_string(),
        refresh_token: reply["refresh_token"].as_str().map(str::to_string).or(old_refresh),
        expires_at: reply["expires_in"].as_i64().map(|s| now + s),
    })
}

fn open_in_browser(url: &str) -> Result<()> {
    // The address holds the client ID, so it is handed to Windows and never shown.
    std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()
        .map(|_| ())
        .map_err(|_| Error::SignIn("Windows could not open your web browser.".into()))
}

/// The page shown in the browser afterwards: plain, accessible HTML with one
/// heading and one sentence. SPEC.md "Sign-in".
fn page(title: &str, sentence: &str) -> String {
    format!(
        "<!doctype html>\n<html lang=\"en-GB\">\n<head>\n<meta charset=\"utf-8\">\n<title>{title}</title>\n</head>\n<body>\n<h1>{title}</h1>\n<p>{sentence}</p>\n</body>\n</html>\n"
    )
}

async fn wait_for_callback(listener: TcpListener, expected_state: &str) -> Result<String> {
    loop {
        let Ok((mut stream, _)) = listener.accept().await else { continue };
        let mut buf = vec![0u8; 8192];
        let n = stream.read(&mut buf).await.unwrap_or(0);
        let request = String::from_utf8_lossy(&buf[..n]);
        let target = request.lines().next().and_then(|l| l.split_whitespace().nth(1)).unwrap_or("/").to_string();
        if !target.starts_with("/callback") {
            // Browsers also ask for /favicon.ico and the like.
            respond(&mut stream, "404 Not Found", &page("Not found", "This is NoteFetch's sign-in helper. You can close this tab.")).await;
            continue;
        }
        let Ok(parsed) = Url::parse(&format!("http://127.0.0.1{target}")) else { continue };
        let (mut code, mut state, mut error) = (None, None, None);
        for (k, v) in parsed.query_pairs() {
            match k.as_ref() {
                "code" => code = Some(v.into_owned()),
                "state" => state = Some(v.into_owned()),
                "error" => error = Some(v.into_owned()),
                _ => {}
            }
        }
        let fail_page = page("Sign-in did not finish", "Close this tab, return to NoteFetch, and try signing in again.");
        if let Some(e) = error {
            respond(&mut stream, "400 Bad Request", &fail_page).await;
            let why = if e == "access_denied" { "access was not allowed in the browser.".to_string() } else { format!("Plaud said \"{e}\".") };
            return Err(Error::SignIn(why));
        }
        if state.as_deref() != Some(expected_state) {
            // Not from this sign-in. Ignore it and keep waiting.
            respond(&mut stream, "400 Bad Request", &fail_page).await;
            continue;
        }
        return match code {
            Some(c) => {
                respond(&mut stream, "200 OK", &page("Signed in to Plaud", "You can close this tab and return to NoteFetch.")).await;
                Ok(c)
            }
            None => {
                respond(&mut stream, "400 Bad Request", &fail_page).await;
                Err(Error::SignIn("Plaud sent no sign-in code.".into()))
            }
        };
    }
}

async fn respond(stream: &mut TcpStream, status: &str, body: &str) {
    let response = format!(
        "HTTP/1.1 {status}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nCache-Control: no-store\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    let _ = stream.write_all(response.as_bytes()).await;
    let _ = stream.flush().await;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_never_show_in_debug_output() {
        let t = Tokens { client_id: "cid".into(), access_token: "secret-a".into(), refresh_token: Some("secret-r".into()), expires_at: None };
        let shown = format!("{t:?}");
        assert!(!shown.contains("secret"));
        assert!(!shown.contains("cid"));
    }

    #[test]
    fn success_page_is_accessible() {
        let p = page("Signed in to Plaud", "You can close this tab and return to NoteFetch.");
        assert!(p.contains("<html lang=\"en-GB\">"));
        assert!(p.contains("<h1>Signed in to Plaud</h1>"));
        assert!(p.contains("<title>Signed in to Plaud</title>"));
    }

    #[test]
    #[ignore = "touches the real machine: DPAPI"]
    fn dpapi_round_trip() {
        let secret = protect(b"hello").unwrap();
        assert_ne!(secret, b"hello");
        assert_eq!(unprotect(&secret).unwrap(), b"hello");
        assert!(unprotect(b"not encrypted").is_none());
    }
}
