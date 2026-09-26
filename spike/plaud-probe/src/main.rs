//! plaud-probe: the milestone 1 spike for NoteFetch.
//!
//! A throwaway command-line program that signs in to Plaud's hosted MCP
//! server and records what the server actually does. Output is plain text.
//! Secrets (tokens, client IDs, authorisation codes, audio links) are never
//! printed; raw replies are saved only under `test-data/`, which git ignores.
//!
//! The sign-in and MCP client code is adapted from plaud-cli
//! (https://github.com/leegonzales/plaud-cli), used under the MIT licence:
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

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use url::Url;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

const MCP_URL: &str = "https://mcp.plaud.ai/mcp";
const PRM_URL: &str = "https://mcp.plaud.ai/.well-known/oauth-protected-resource/mcp";
const PRM_URL_ROOT: &str = "https://mcp.plaud.ai/.well-known/oauth-protected-resource";
const PROTOCOL_VERSION: &str = "2025-06-18";
const CLIENT_NAME: &str = "NoteFetch probe";
const CALLBACK_TIMEOUT: Duration = Duration::from_secs(300);

fn fail(message: impl Into<String>) -> Box<dyn std::error::Error> {
    message.into().into()
}

// ---------------------------------------------------------------------------
// Files under test-data

fn data_dir() -> PathBuf {
    // The spike runs from spike/plaud-probe; test-data is at the repository root.
    let here = Path::new(env!("CARGO_MANIFEST_DIR"));
    here.join("..").join("..").join("test-data").join("milestone-1")
}

fn save(name: &str, value: &Value) -> Result<PathBuf> {
    let dir = data_dir();
    std::fs::create_dir_all(&dir)?;
    let path = dir.join(name);
    std::fs::write(&path, serde_json::to_string_pretty(value)?)?;
    Ok(path)
}

fn tokens_path() -> PathBuf {
    data_dir().join("tokens.json")
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// A secret shown only by its length, so two can be told apart without
/// revealing either.
fn hidden(secret: &str) -> String {
    format!("(hidden, {} characters)", secret.len())
}

/// Replace any string value under a key that looks secret, recursively, so a
/// raw reply can be saved and shared without its secrets.
fn redact(value: &mut Value) {
    const SECRET_KEYS: [&str; 8] = [
        "data_link",
        "access_token",
        "refresh_token",
        "id_token",
        "client_id",
        "client_secret",
        "presigned_url",
        "registration_access_token",
    ];
    match value {
        Value::Object(map) => {
            for (key, inner) in map.iter_mut() {
                if SECRET_KEYS.contains(&key.as_str()) {
                    if let Value::String(s) = inner {
                        *inner = Value::String(hidden(s));
                    }
                } else if key.ends_with("_url") || key == "url" {
                    if let Value::String(s) = inner
                        && (s.contains("Signature") || s.contains("X-Amz") || s.contains("sig="))
                    {
                        *inner = Value::String(format!("(signed link {})", hidden(s)));
                    }
                } else {
                    redact(inner);
                }
            }
        }
        Value::Array(items) => items.iter_mut().for_each(redact),
        _ => {}
    }
}

// ---------------------------------------------------------------------------
// Tokens, kept in test-data only (the real app uses DPAPI)

#[derive(Clone)]
struct Tokens {
    client_id: String,
    access_token: String,
    refresh_token: Option<String>,
    expires_at: Option<u64>,
}

fn save_tokens(t: &Tokens) -> Result<()> {
    std::fs::create_dir_all(data_dir())?;
    let v = json!({
        "client_id": t.client_id,
        "access_token": t.access_token,
        "refresh_token": t.refresh_token,
        "expires_at": t.expires_at,
    });
    std::fs::write(tokens_path(), serde_json::to_string(&v)?)?;
    Ok(())
}

fn load_tokens() -> Result<Tokens> {
    let text = std::fs::read_to_string(tokens_path())
        .map_err(|_| fail("Not signed in. Run: cargo run -- login"))?;
    let v: Value = serde_json::from_str(&text)?;
    Ok(Tokens {
        client_id: v["client_id"].as_str().unwrap_or_default().to_string(),
        access_token: v["access_token"].as_str().unwrap_or_default().to_string(),
        refresh_token: v["refresh_token"].as_str().map(str::to_string),
        expires_at: v["expires_at"].as_u64(),
    })
}

// ---------------------------------------------------------------------------
// Discovery

struct Endpoints {
    authorization: String,
    token: String,
    registration: Option<String>,
}

async fn get_json(http: &reqwest::Client, url: &str) -> Result<(u16, Value)> {
    let resp = http.get(url).send().await?;
    let status = resp.status().as_u16();
    let text = resp.text().await?;
    let value = serde_json::from_str(&text).unwrap_or(Value::Null);
    Ok((status, value))
}

async fn discover(http: &reqwest::Client, verbose: bool) -> Result<Endpoints> {
    let (status, prm) = get_json(http, PRM_URL).await?;
    let prm = if status == 200 {
        prm
    } else {
        if verbose {
            println!("{PRM_URL} returned HTTP {status}; trying the root form.");
        }
        get_json(http, PRM_URL_ROOT).await?.1
    };
    let issuer = prm["authorization_servers"][0]
        .as_str()
        .ok_or_else(|| fail("The protected resource metadata names no authorisation server."))?
        .trim_end_matches('/')
        .to_string();
    let as_url = format!("{issuer}/.well-known/oauth-authorization-server");
    let (_, meta) = get_json(http, &as_url).await?;

    if verbose {
        save("discovery-protected-resource.json", &prm)?;
        save("discovery-authorisation-server.json", &meta)?;
        println!("Protected resource: {}", prm["resource"]);
        println!("Authorisation server: {issuer}");
        println!("Bearer methods: {}", prm["bearer_methods_supported"]);
        println!("Scopes offered by the resource: {}", prm["scopes_supported"]);
        for key in [
            "authorization_endpoint",
            "token_endpoint",
            "registration_endpoint",
            "revocation_endpoint",
            "scopes_supported",
            "response_types_supported",
            "grant_types_supported",
            "code_challenge_methods_supported",
            "token_endpoint_auth_methods_supported",
        ] {
            println!("{key}: {}", meta[key]);
        }
        println!("Saved both documents to {}", data_dir().display());
    }

    Ok(Endpoints {
        authorization: meta["authorization_endpoint"]
            .as_str()
            .ok_or_else(|| fail("No authorization_endpoint."))?
            .to_string(),
        token: meta["token_endpoint"]
            .as_str()
            .ok_or_else(|| fail("No token_endpoint."))?
            .to_string(),
        registration: meta["registration_endpoint"].as_str().map(str::to_string),
    })
}

// ---------------------------------------------------------------------------
// Sign-in

fn random_url_safe(bytes: usize) -> String {
    let mut raw = vec![0u8; bytes];
    rand::fill(&mut raw[..]);
    URL_SAFE_NO_PAD.encode(raw)
}

fn pkce_pair() -> (String, String) {
    let verifier = random_url_safe(32);
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

fn open_browser(url: &str) -> Result<()> {
    // The URL holds the client ID, so it is handed to Windows, never printed.
    std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn()?;
    Ok(())
}

async fn login(http: &reqwest::Client) -> Result<()> {
    let ep = discover(http, false).await?;
    let registration = ep
        .registration
        .clone()
        .ok_or_else(|| fail("The server offers no registration endpoint."))?;

    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    let redirect_uri = format!("http://127.0.0.1:{port}/callback");
    println!("Listening for the browser on {redirect_uri}");

    let body = json!({
        "client_name": CLIENT_NAME,
        "redirect_uris": [redirect_uri],
        "grant_types": ["authorization_code", "refresh_token"],
        "response_types": ["code"],
        "token_endpoint_auth_method": "none",
    });
    let resp = http.post(&registration).json(&body).send().await?;
    let status = resp.status().as_u16();
    let mut reg: Value = serde_json::from_str(&resp.text().await?).unwrap_or(Value::Null);
    println!("Registration: HTTP {status}");
    let client_id = reg["client_id"]
        .as_str()
        .ok_or_else(|| fail(format!("Registration was refused (HTTP {status}).")))?
        .to_string();
    redact(&mut reg);
    let keys: Vec<&String> = reg.as_object().map(|m| m.keys().collect()).unwrap_or_default();
    println!("Registration reply fields: {keys:?}");
    println!("Registered redirect addresses: {}", reg["redirect_uris"]);
    println!("Registered auth method: {}", reg["token_endpoint_auth_method"]);
    println!("Client secret expiry: {}", reg["client_secret_expires_at"]);
    save("registration-reply.json", &reg)?;

    let (verifier, challenge) = pkce_pair();
    let state = random_url_safe(16);
    let mut auth_url = Url::parse(&ep.authorization)?;
    auth_url
        .query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", &client_id)
        .append_pair("redirect_uri", &redirect_uri)
        .append_pair("state", &state)
        .append_pair("code_challenge", &challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("resource", MCP_URL);

    println!("Opening Plaud's sign-in page in your browser. Sign in, then come back here.");
    println!("Waiting up to five minutes.");
    open_browser(auth_url.as_str())?;

    let code = tokio::time::timeout(CALLBACK_TIMEOUT, wait_for_callback(listener, &state))
        .await
        .map_err(|_| fail("Timed out after five minutes waiting for the browser."))??;
    println!("The browser returned an authorisation code.");

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
    let status = resp.status().as_u16();
    let reply: Value = serde_json::from_str(&resp.text().await?).unwrap_or(Value::Null);
    println!("Token exchange: HTTP {status}");
    let tokens = describe_token_reply(&reply, client_id, None)?;
    save_tokens(&tokens)?;
    println!("Signed in. Tokens saved under test-data, which git ignores.");
    Ok(())
}

/// Print what a token reply contains, without the tokens, and build Tokens.
fn describe_token_reply(reply: &Value, client_id: String, old_refresh: Option<String>) -> Result<Tokens> {
    let Some(access) = reply["access_token"].as_str() else {
        let code = reply["error"].as_str().unwrap_or("unknown");
        return Err(fail(format!("No access token in the reply. Error code: {code}")));
    };
    let fields: Vec<&String> = reply.as_object().map(|m| m.keys().collect()).unwrap_or_default();
    println!("Token reply fields: {fields:?}");
    println!("token_type: {}", reply["token_type"]);
    println!("scope: {}", reply["scope"]);
    let expires_in = reply["expires_in"].as_u64();
    match expires_in {
        Some(s) => println!("Access token lasts {s} seconds ({:.1} hours).", s as f64 / 3600.0),
        None => println!("No expires_in given."),
    }
    for key in ["refresh_token_expires_in", "refresh_expires_in"] {
        if let Some(s) = reply[key].as_u64() {
            println!("{key}: {s} seconds ({:.1} days).", s as f64 / 86400.0);
        }
    }
    let new_refresh = reply["refresh_token"].as_str().map(str::to_string);
    match (&new_refresh, &old_refresh) {
        (Some(r), Some(o)) if r == o => println!("Refresh token: the same one returned."),
        (Some(r), Some(_)) => println!("Refresh token: a new one {} (rotated).", hidden(r)),
        (Some(r), None) => println!("Refresh token: {}", hidden(r)),
        (None, _) => println!("Refresh token: none in this reply."),
    }
    println!("Access token: {}", hidden(access));
    Ok(Tokens {
        client_id,
        access_token: access.to_string(),
        refresh_token: new_refresh.or(old_refresh),
        expires_at: expires_in.map(|s| now() + s),
    })
}

const PAGE_OK: &str = "<!doctype html>\n<html lang=\"en-GB\">\n<head><meta charset=\"utf-8\"><title>Signed in to Plaud</title></head>\n<body>\n<h1>Signed in to Plaud</h1>\n<p>You can close this tab and return to the plaud-probe window.</p>\n</body>\n</html>\n";

fn page_failed(reason: &str) -> String {
    format!("<!doctype html>\n<html lang=\"en-GB\">\n<head><meta charset=\"utf-8\"><title>Sign-in did not finish</title></head>\n<body>\n<h1>Sign-in did not finish</h1>\n<p>{reason} Close this tab and try again from the plaud-probe window.</p>\n</body>\n</html>\n")
}

async fn wait_for_callback(listener: TcpListener, expected_state: &str) -> Result<String> {
    loop {
        let (mut stream, _) = listener.accept().await?;
        let mut buf = vec![0u8; 8192];
        let n = stream.read(&mut buf).await?;
        let request = String::from_utf8_lossy(&buf[..n]);
        let target = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or("/")
            .to_string();
        if !target.starts_with("/callback") {
            respond(&mut stream, 404, "Not Found", &page_failed("This address is not the sign-in page.")).await?;
            continue;
        }
        let parsed = Url::parse(&format!("http://127.0.0.1{target}"))?;
        let (mut code, mut state, mut error) = (None, None, None);
        for (k, v) in parsed.query_pairs() {
            match k.as_ref() {
                "code" => code = Some(v.into_owned()),
                "state" => state = Some(v.into_owned()),
                "error" => error = Some(v.into_owned()),
                _ => {}
            }
        }
        if let Some(e) = error {
            respond(&mut stream, 400, "Bad Request", &page_failed("Plaud did not grant access.")).await?;
            return Err(fail(format!("Plaud refused the sign-in. Error code: {e}")));
        }
        if state.as_deref() != Some(expected_state) {
            respond(&mut stream, 400, "Bad Request", &page_failed("The reply did not match this sign-in.")).await?;
            return Err(fail("The browser's reply did not match this sign-in, so it was ignored."));
        }
        return match code {
            Some(c) => {
                respond(&mut stream, 200, "OK", PAGE_OK).await?;
                Ok(c)
            }
            None => {
                respond(&mut stream, 400, "Bad Request", &page_failed("Plaud sent no authorisation code.")).await?;
                Err(fail("Plaud sent no authorisation code."))
            }
        };
    }
}

async fn respond(stream: &mut tokio::net::TcpStream, status: u16, reason: &str, body: &str) -> Result<()> {
    let response = format!(
        "HTTP/1.1 {status} {reason}\r\nContent-Type: text/html; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(response.as_bytes()).await?;
    stream.flush().await?;
    Ok(())
}

async fn refresh(http: &reqwest::Client, tokens: &Tokens) -> Result<Tokens> {
    let refresh_token = tokens
        .refresh_token
        .clone()
        .ok_or_else(|| fail("No refresh token stored. Sign in again."))?;
    let ep = discover(http, false).await?;
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
    let status = resp.status().as_u16();
    let reply: Value = serde_json::from_str(&resp.text().await?).unwrap_or(Value::Null);
    println!("Refresh: HTTP {status}");
    let fresh = describe_token_reply(&reply, tokens.client_id.clone(), Some(refresh_token))?;
    save_tokens(&fresh)?;
    Ok(fresh)
}

// ---------------------------------------------------------------------------
// MCP client

struct Mcp {
    http: reqwest::Client,
    tokens: Tokens,
    session: Option<String>,
    next_id: i64,
}

impl Mcp {
    async fn connect(http: reqwest::Client) -> Result<Self> {
        let tokens = load_tokens()?;
        if let Some(at) = tokens.expires_at {
            let left = at as i64 - now() as i64;
            println!("Stored access token expires in {left} seconds.");
        }
        let mut mcp = Mcp { http, tokens, session: None, next_id: 0 };
        mcp.initialize().await?;
        Ok(mcp)
    }

    async fn post_once(&self, body: &Value) -> Result<reqwest::Response> {
        let mut req = self
            .http
            .post(MCP_URL)
            .bearer_auth(&self.tokens.access_token)
            .header("Accept", "application/json, text/event-stream")
            .header("MCP-Protocol-Version", PROTOCOL_VERSION)
            .json(body);
        if let Some(sid) = &self.session {
            req = req.header("Mcp-Session-Id", sid);
        }
        Ok(req.send().await?)
    }

    async fn initialize(&mut self) -> Result<()> {
        self.next_id += 1;
        let id = self.next_id;
        let body = json!({
            "jsonrpc": "2.0", "id": id, "method": "initialize",
            "params": {
                "protocolVersion": PROTOCOL_VERSION,
                "capabilities": {},
                "clientInfo": { "name": "notefetch-probe", "version": env!("CARGO_PKG_VERSION") }
            }
        });
        let mut resp = self.post_once(&body).await?;
        if resp.status().as_u16() == 401 {
            println!("initialize: HTTP 401, refreshing the token.");
            self.tokens = refresh(&self.http, &self.tokens).await?;
            resp = self.post_once(&body).await?;
        }
        let status = resp.status().as_u16();
        self.session = resp
            .headers()
            .get("Mcp-Session-Id")
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);
        let reply = read_reply(resp, id).await?;
        if status != 200 {
            return Err(fail(format!("initialize failed with HTTP {status}.")));
        }
        println!(
            "initialize: HTTP {status}, session {}, server {}, protocol {}",
            if self.session.is_some() { "given" } else { "not given" },
            reply["result"]["serverInfo"],
            reply["result"]["protocolVersion"],
        );
        save("initialize-reply.json", &reply)?;
        let note = json!({ "jsonrpc": "2.0", "method": "notifications/initialized" });
        let _ = self.post_once(&note).await?;
        Ok(())
    }

    /// Send a request, refreshing once and starting a new session on 401.
    async fn request(&mut self, method: &str, params: Value) -> Result<Value> {
        self.next_id += 1;
        let id = self.next_id;
        let body = json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params });
        let mut resp = self.post_once(&body).await?;
        if resp.status().as_u16() == 401 {
            println!("{method}: HTTP 401, refreshing the token and starting a new session.");
            self.tokens = refresh(&self.http, &self.tokens).await?;
            self.session = None;
            self.initialize().await?;
            resp = self.post_once(&body).await?;
        }
        let status = resp.status().as_u16();
        if status != 200 {
            let text = resp.text().await.unwrap_or_default();
            let short: String = text.chars().take(200).collect();
            return Err(fail(format!("{method} failed with HTTP {status}. Start of reply: {short}")));
        }
        read_reply(resp, id).await
    }

    /// Call a tool; returns (is_error, payload).
    async fn tool(&mut self, name: &str, arguments: Value) -> Result<(bool, Value)> {
        let reply = self
            .request("tools/call", json!({ "name": name, "arguments": arguments }))
            .await?;
        if let Some(err) = reply.get("error") {
            return Err(fail(format!("{name}: protocol error {}", err)));
        }
        let result = &reply["result"];
        let is_error = result["isError"].as_bool().unwrap_or(false);
        if let Some(s) = result.get("structuredContent").filter(|v| !v.is_null()) {
            return Ok((is_error, s.clone()));
        }
        let mut text = String::new();
        for item in result["content"].as_array().into_iter().flatten() {
            if let Some(t) = item["text"].as_str() {
                text.push_str(t);
            }
        }
        let inner = unwrap_envelope(&text).unwrap_or(&text);
        let payload = serde_json::from_str(inner).unwrap_or(Value::String(text.clone()));
        Ok((is_error, payload))
    }
}

/// Plaud wraps recording data in `<untrusted-user-data-XXXX ...> ... </untrusted-user-data-XXXX>`,
/// after a sentence saying the contents are data, not instructions. Return what is inside.
fn unwrap_envelope(text: &str) -> Option<&str> {
    const PREFIX: &str = "<untrusted-user-data-";
    let open = text.find(PREFIX)?;
    let after_prefix = &text[open + 1..];
    let name_len = after_prefix.find(|c: char| c.is_whitespace() || c == '>')?;
    let tag = &after_prefix[..name_len];
    // The preamble sentence names the tag too; the real opening tag starts a line.
    let opening = open + text[open..].find(&format!("\n<{tag}"))? + 1;
    let body_start = opening + text[opening..].find('>')? + 1;
    let close = format!("</{tag}>");
    let body_end = body_start + text[body_start..].find(&close)?;
    Some(text[body_start..body_end].trim())
}

async fn read_reply(resp: reqwest::Response, id: i64) -> Result<Value> {
    let sse = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|ct| ct.contains("text/event-stream"));
    let text = resp.text().await?;
    let messages: Vec<Value> = if sse {
        let mut out = Vec::new();
        let mut data = String::new();
        for line in text.lines().chain(std::iter::once("")) {
            if let Some(rest) = line.strip_prefix("data:") {
                data.push_str(rest.trim_start());
                data.push('\n');
            } else if line.trim().is_empty() && !data.is_empty() {
                if let Ok(v) = serde_json::from_str::<Value>(data.trim()) {
                    out.push(v);
                }
                data.clear();
            }
        }
        out
    } else if text.trim().is_empty() {
        return Ok(Value::Null);
    } else {
        match serde_json::from_str::<Value>(&text)? {
            Value::Array(items) => items,
            other => vec![other],
        }
    };
    messages
        .into_iter()
        .find(|m| m["id"] == json!(id))
        .ok_or_else(|| fail(format!("No reply matching request {id}.")))
}

// ---------------------------------------------------------------------------
// Probes

fn items(payload: &Value) -> Vec<Value> {
    payload["data"]
        .as_array()
        .or_else(|| payload.as_array())
        .cloned()
        .unwrap_or_default()
}

fn keys(v: &Value) -> Vec<String> {
    v.as_object().map(|m| m.keys().cloned().collect()).unwrap_or_default()
}

/// Field names that suggest folders or tags.
fn organisation_fields(v: &Value) -> Vec<String> {
    keys(v)
        .into_iter()
        .filter(|k| {
            let k = k.to_lowercase();
            ["folder", "tag", "category", "group", "label", "dir", "project"]
                .iter()
                .any(|w| k.contains(w))
        })
        .collect()
}

fn describe_payload(name: &str, is_error: bool, payload: &Value) {
    if is_error {
        let text = payload.as_str().map(str::to_string).unwrap_or_else(|| payload.to_string());
        let short: String = text.chars().take(300).collect();
        println!("{name}: the tool reported an error: {short}");
        return;
    }
    let shape = match payload {
        Value::Object(_) => format!("object with fields {:?}", keys(payload)),
        Value::Array(a) => format!("array of {}", a.len()),
        Value::String(s) => format!("text of {} characters", s.len()),
        other => format!("{other}"),
    };
    println!("{name}: {shape}");
}

async fn probe(http: reqwest::Client) -> Result<()> {
    let mut mcp = Mcp::connect(http.clone()).await?;

    println!();
    println!("Step 1: tools/list");
    let tools = mcp.request("tools/list", json!({})).await?;
    let path = save("tools-list.json", &tools)?;
    for tool in tools["result"]["tools"].as_array().into_iter().flatten() {
        let params: Vec<String> = keys(&tool["inputSchema"]["properties"]);
        println!("Tool {} takes {:?}", tool["name"], params);
    }
    println!("Saved to {}", path.display());

    println!();
    println!("Step 2: get_current_user (this is also the plan check)");
    match mcp.tool("get_current_user", json!({})).await {
        Ok((is_error, mut payload)) => {
            describe_payload("get_current_user", is_error, &payload);
            redact(&mut payload);
            save("get-current-user.json", &payload)?;
            println!("Saved, but it holds your account details; do not share it.");
        }
        Err(e) => println!("get_current_user failed: {e}"),
    }

    println!();
    println!("Step 3: list_files page sizes");
    for size in [1u32, 5, 9, 10, 20, 50, 100, 200, 500, 1000] {
        match mcp.tool("list_files", json!({ "page": 1, "page_size": size })).await {
            Ok((true, payload)) => describe_payload(&format!("page_size {size}"), true, &payload),
            Ok((false, payload)) => {
                let n = items(&payload).len();
                let extra: Vec<String> = keys(&payload).into_iter().filter(|k| k != "data").collect();
                println!("page_size {size}: {n} recordings; other fields {extra:?}");
                for k in ["total", "total_count", "count", "has_more", "page", "page_size"] {
                    if !payload[k].is_null() {
                        println!("  {k}: {}", payload[k]);
                    }
                }
            }
            Err(e) => println!("page_size {size}: failed: {e}"),
        }
    }

    println!();
    println!("Step 4: walk every page at 100 per page");
    let mut all = Vec::new();
    for page in 1..=50u32 {
        let (is_error, payload) = mcp.tool("list_files", json!({ "page": page, "page_size": 100 })).await?;
        if is_error {
            describe_payload(&format!("page {page}"), true, &payload);
            break;
        }
        let batch = items(&payload);
        let n = batch.len();
        println!("page {page}: {n} recordings");
        all.extend(batch);
        if n < 100 {
            break;
        }
    }
    println!("Total recordings seen: {}", all.len());
    let mut ids: Vec<&str> = all.iter().filter_map(|r| r["id"].as_str()).collect();
    ids.sort_unstable();
    ids.dedup();
    println!("Distinct IDs: {}", ids.len());
    save("list-files-all.json", &Value::Array(all.clone()))?;
    println!("Saved every listed recording to list-files-all.json.");

    let Some(first) = all.first().cloned() else {
        println!("No recordings, so the remaining steps are skipped.");
        return Ok(());
    };
    println!("Fields on a listed recording: {:?}", keys(&first));
    println!("Fields suggesting folders or tags: {:?}", organisation_fields(&first));

    println!();
    println!("Step 5: are times UTC? Recordings whose name is still a date and time:");
    let mut shown = 0;
    for r in &all {
        let name = r["name"].as_str().unwrap_or_default();
        let b = name.as_bytes();
        let looks_dated = b.len() >= 16 && b[4] == b'-' && b[7] == b'-' && b[10] == b' ' && b[13] == b':';
        if looks_dated && shown < 5 {
            println!("  name {name:?}  start_at {}  created_at {}", r["start_at"], r["created_at"]);
            shown += 1;
        }
    }
    if shown == 0 {
        println!("  None found. First recording: start_at {} created_at {}", first["start_at"], first["created_at"]);
    }

    let id = first["id"].as_str().unwrap_or_default().to_string();
    println!();
    println!("Step 6: get_file, get_transcript and get_note on the newest recording");
    let (is_error, mut file) = mcp.tool("get_file", json!({ "file_id": id })).await?;
    describe_payload("get_file", is_error, &file);
    let link = file["presigned_url"].as_str().map(str::to_string);
    println!("presigned_url present: {}", link.is_some());
    println!("Fields suggesting folders or tags: {:?}", organisation_fields(&file));
    redact(&mut file);
    save("get-file.json", &file)?;

    let (is_error, transcript) = mcp.tool("get_transcript", json!({ "file_id": id })).await?;
    describe_payload("get_transcript", is_error, &transcript);
    if let Some(blocks) = transcript.as_array() {
        for b in blocks {
            println!("  block data_type {} fields {:?}", b["data_type"], keys(b));
        }
    }
    save("get-transcript.json", &transcript)?;

    let (is_error, note) = mcp.tool("get_note", json!({ "file_id": id })).await?;
    describe_payload("get_note", is_error, &note);
    if let Some(blocks) = note.as_array() {
        for b in blocks {
            println!(
                "  block data_type {} tab {} title {}",
                b["data_type"], b["data_tab_name"], b["data_title"]
            );
        }
    }
    save("get-note.json", &note)?;

    if let Some(link) = link {
        println!();
        println!("Step 7: download the audio");
        download(&http, &link, &id).await?;
    }
    println!();
    println!("Probe finished. Raw replies are in {}", data_dir().display());
    Ok(())
}

/// `raw <tool> [json arguments] [file name]`: call one tool, save the whole
/// tools/call result (redacted) and print its shape.
async fn raw(http: reqwest::Client) -> Result<()> {
    let args: Vec<String> = std::env::args().skip(2).collect();
    let tool = args.first().ok_or_else(|| fail("Usage: raw <tool> [json arguments] [file name]"))?;
    let arguments: Value = match args.get(1) {
        Some(a) => serde_json::from_str(a)?,
        None => json!({}),
    };
    let file = args.get(2).cloned().unwrap_or_else(|| format!("raw-{tool}.json"));
    let mut mcp = Mcp::connect(http).await?;
    let mut reply = mcp
        .request("tools/call", json!({ "name": tool, "arguments": arguments }))
        .await?;
    let result = &reply["result"];
    println!("isError: {}", result["isError"]);
    println!("result fields: {:?}", keys(result));
    for item in result["content"].as_array().into_iter().flatten() {
        let text = item["text"].as_str().unwrap_or_default();
        println!("content item type {} of {} characters", item["type"], text.len());
        if let Ok(v) = serde_json::from_str::<Value>(text) {
            describe_payload("  parsed as JSON", false, &v);
        }
    }
    redact(&mut reply);
    let path = save(&file, &reply)?;
    println!("Saved to {}", path.display());
    Ok(())
}

async fn download(http: &reqwest::Client, link: &str, id: &str) -> Result<()> {
    let mut resp = http.get(link).send().await?;
    let status = resp.status().as_u16();
    let content_type = resp
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("(none)")
        .to_string();
    let length = resp.content_length();
    println!("Audio: HTTP {status}, content type {content_type}, Content-Length {length:?}");
    for h in ["content-disposition", "accept-ranges", "etag", "last-modified"] {
        if let Some(v) = resp.headers().get(h).and_then(|v| v.to_str().ok()) {
            println!("  {h}: {v}");
        }
    }
    if status != 200 {
        return Ok(());
    }
    let dir = data_dir().join("audio");
    std::fs::create_dir_all(&dir)?;
    let safe_id: String = id.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let path = dir.join(format!("{safe_id}.download"));
    let mut file = tokio::fs::File::create(&path).await?;
    let mut written: u64 = 0;
    while let Some(chunk) = resp.chunk().await? {
        file.write_all(&chunk).await?;
        written += chunk.len() as u64;
    }
    file.flush().await?;
    let mut head = [0u8; 12];
    let mut f = std::fs::File::open(&path)?;
    let got = std::io::Read::read(&mut f, &mut head)?;
    println!("Wrote {written} bytes to {}", path.display());
    println!("First bytes: {:02X?} (ID3 or FF Fx means MP3, ftyp at 4 means M4A, OggS means Ogg)", &head[..got]);
    Ok(())
}

// ---------------------------------------------------------------------------

#[tokio::main]
async fn main() {
    let command = std::env::args().nth(1).unwrap_or_default();
    let http = match reqwest::Client::builder()
        .user_agent(concat!("notefetch-probe/", env!("CARGO_PKG_VERSION")))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            println!("Could not start the web client: {e}");
            std::process::exit(2);
        }
    };
    let outcome = match command.as_str() {
        "discover" => discover(&http, true).await.map(|_| ()),
        "login" => login(&http).await,
        "probe" => probe(http).await,
        "refresh" => match load_tokens() {
            Ok(t) => refresh(&http, &t).await.map(|_| ()),
            Err(e) => Err(e),
        },
        "raw" => raw(http).await,
        "status" => load_tokens().map(|t| match t.expires_at {
            Some(at) => println!("Access token expires in {} seconds.", at as i64 - now() as i64),
            None => println!("No expiry recorded."),
        }),
        _ => {
            println!("Commands: discover, login, probe, refresh, status");
            Ok(())
        }
    };
    if let Err(e) = outcome {
        println!("Stopped: {e}");
        std::process::exit(1);
    }
}
