//! Downloading a recording's audio. SPEC.md "Downloads and network behaviour".
//!
//! Audio streams to `<name>.ogg.part` and is renamed only when complete and
//! matching the size the server gave. An expired or refused link is replaced
//! once by asking Plaud for a fresh one. Network failures, 429 and 5xx are
//! retried three times with increasing delay. Cancelling stops at once and
//! leaves no `.part` file behind.

use std::path::{Path, PathBuf};
use std::time::Duration;

use tokio::io::AsyncWriteExt;

use crate::cancel::Cancel;
use crate::error::{Error, Result, disk};
use crate::mcp::Client;

const RETRIES: u32 = 3;

pub async fn audio(http: &reqwest::Client, client: &Client, id: &str, link: &str, dest: &Path, cancel: &Cancel) -> Result<()> {
    let part = part_path(dest);
    let result = attempts(http, client, id, link, dest, &part, cancel).await;
    if result.is_err() {
        let _ = tokio::fs::remove_file(&part).await;
    }
    result
}

fn part_path(dest: &Path) -> PathBuf {
    let mut p = dest.as_os_str().to_owned();
    p.push(".part");
    PathBuf::from(p)
}

async fn attempts(http: &reqwest::Client, client: &Client, id: &str, link: &str, dest: &Path, part: &Path, cancel: &Cancel) -> Result<()> {
    let mut link = link.to_string();
    let mut refreshed_link = false;
    let mut attempt = 0;
    loop {
        cancel.check()?;
        match once(http, &link, part, cancel).await {
            Ok(()) => break,
            Err(Attempt::LinkRefused) if !refreshed_link => {
                refreshed_link = true;
                let fresh = crate::plaud::detail(client, id).await?;
                link = fresh
                    .audio_link
                    .ok_or_else(|| Error::Plaud("it no longer offers the audio for this recording.".into()))?;
            }
            Err(Attempt::LinkRefused) => {
                return Err(Error::Plaud("it refused the audio download twice.".into()));
            }
            Err(Attempt::Retry(e)) => {
                if attempt >= RETRIES {
                    return Err(e);
                }
                attempt += 1;
                tokio::select! {
                    _ = tokio::time::sleep(Duration::from_secs(1 << attempt)) => {}
                    _ = cancel.cancelled() => return Err(Error::Cancelled),
                }
            }
            Err(Attempt::Final(e)) => return Err(e),
        }
    }
    tokio::fs::rename(part, dest).await.map_err(|e| disk("save", dest, &e))
}

enum Attempt {
    LinkRefused,
    Retry(Error),
    Final(Error),
}

async fn once(http: &reqwest::Client, link: &str, part: &Path, cancel: &Cancel) -> std::result::Result<(), Attempt> {
    let mut resp = http.get(link).send().await.map_err(|e| Attempt::Retry(e.into()))?;
    match resp.status().as_u16() {
        200 => {}
        400 | 401 | 403 | 404 | 410 => return Err(Attempt::LinkRefused),
        429 => return Err(Attempt::Retry(Error::Plaud("its storage asked NoteFetch to slow down.".into()))),
        s if s >= 500 => return Err(Attempt::Retry(Error::Plaud(format!("its storage had an internal error (HTTP {s}).")))),
        s => return Err(Attempt::Final(Error::Plaud(format!("its storage refused the download (HTTP {s}).")))),
    }
    let expected = resp.content_length();
    let mut file = tokio::fs::File::create(part).await.map_err(|e| Attempt::Final(disk("create", part, &e)))?;
    let mut written: u64 = 0;
    loop {
        let chunk = tokio::select! {
            c = resp.chunk() => c.map_err(|e| Attempt::Retry(e.into()))?,
            _ = cancel.cancelled() => return Err(Attempt::Final(Error::Cancelled)),
        };
        let Some(chunk) = chunk else { break };
        file.write_all(&chunk).await.map_err(|e| Attempt::Final(disk("write", part, &e)))?;
        written += chunk.len() as u64;
    }
    file.flush().await.map_err(|e| Attempt::Final(disk("write", part, &e)))?;
    drop(file);
    if let Some(n) = expected
        && n != written
    {
        return Err(Attempt::Retry(Error::Network(format!("the download stopped after {written} of {n} bytes."))));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn part_file_sits_beside_the_audio() {
        assert_eq!(part_path(Path::new(r"C:\x\Lecture, 2026-09-24 1430.ogg")), PathBuf::from(r"C:\x\Lecture, 2026-09-24 1430.ogg.part"));
    }
}
