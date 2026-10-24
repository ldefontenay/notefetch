//! Plaud's recordings as typed values, from the replies seen in milestone 1
//! (`test-milestone-1.txt`), and the text files made from them.

use serde_json::{Value, json};

use crate::clock::{self, Stamp};
use crate::error::{Error, Result};
use crate::mcp::Client;

/// Largest page `list_files` accepts (proven: 10 to 100).
const PAGE_SIZE: usize = 100;
/// Safety stop: 100 pages is 10,000 recordings.
const MAX_PAGES: usize = 100;

/// A recording as listed.
#[derive(Debug, Clone, PartialEq)]
pub struct Listed {
    pub id: String,
    pub name: String,
    /// UTC.
    pub start: Stamp,
    pub duration_ms: u64,
}

impl Listed {
    fn from_json(v: &Value) -> Option<Listed> {
        Some(Listed {
            id: v["id"].as_str().filter(|s| !s.is_empty())?.to_string(),
            name: v["name"].as_str().unwrap_or_default().to_string(),
            start: ["start_at", "created_at"].iter().find_map(|k| Stamp::parse(v[*k].as_str()?))?,
            duration_ms: v["duration"].as_u64().unwrap_or(0),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub start_ms: u64,
    pub speaker: Option<String>,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Topic {
    pub start_ms: u64,
    pub topic: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Note {
    pub tab: String,
    pub markdown: String,
}

/// Everything about one recording, from `get_file`.
#[derive(Clone, PartialEq)]
pub struct Detail {
    pub listed: Listed,
    /// Signed link to the audio, valid 24 hours. A secret: never logged.
    pub audio_link: Option<String>,
    pub notes: Vec<Note>,
    pub transcript: Option<Vec<Segment>>,
    pub cleaned_transcript: Option<Vec<Segment>>,
    pub outline: Option<Vec<Topic>>,
    pub highlights: Option<Value>,
    /// Blocks Plaud listed but sent without content, to fetch with
    /// `get_transcript`.
    pub missing_blocks: Vec<String>,
}

// No Debug derive: the audio link is a secret.
impl std::fmt::Debug for Detail {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Detail").field("listed", &self.listed).field("notes", &self.notes.len()).finish_non_exhaustive()
    }
}

/// Every recording, newest first, paging 100 at a time until a short page.
pub async fn list_all(client: &Client) -> Result<Vec<Listed>> {
    let mut all = Vec::new();
    for page in 1..=MAX_PAGES {
        let reply = client.call_tool("list_files", json!({ "page": page, "page_size": PAGE_SIZE })).await?;
        let items = reply["data"].as_array().cloned().unwrap_or_default();
        let count = items.len();
        all.extend(items.iter().filter_map(Listed::from_json));
        if count < PAGE_SIZE {
            break;
        }
    }
    // A recording that moved between pages while paging is listed twice.
    let mut seen = std::collections::HashSet::new();
    all.retain(|r: &Listed| seen.insert(r.id.clone()));
    Ok(all)
}

/// Everything about one recording, fetching any block `get_file` left empty.
pub async fn detail(client: &Client, id: &str) -> Result<Detail> {
    let reply = client.call_tool("get_file", json!({ "file_id": id })).await?;
    let mut d = parse_detail(&reply).ok_or(Error::Unreadable)?;
    for block in std::mem::take(&mut d.missing_blocks) {
        let content = fetch_block(client, id, &block).await?;
        apply_block(&mut d, &block, &content);
    }
    Ok(d)
}

/// Page through `get_transcript` for one block and return it in the same
/// form `get_file` uses.
async fn fetch_block(client: &Client, id: &str, block: &str) -> Result<Value> {
    let mut items = Vec::new();
    let mut cursor: Option<String> = None;
    for _ in 0..1000 {
        let mut args = json!({ "file_id": id, "block": block });
        if let Some(c) = &cursor {
            args["cursor"] = json!(c);
        }
        let page = client.call_tool("get_transcript", args).await?;
        for key in ["segments", "marks", "items", "outline"] {
            if let Some(a) = page[key].as_array() {
                items.extend(a.iter().cloned());
            }
        }
        cursor = page["next_cursor"].as_str().filter(|c| !c.is_empty()).map(str::to_string);
        if cursor.is_none() {
            break;
        }
    }
    Ok(Value::Array(items))
}

pub fn parse_detail(v: &Value) -> Option<Detail> {
    let mut d = Detail {
        listed: Listed::from_json(v)?,
        audio_link: v["presigned_url"].as_str().filter(|s| !s.is_empty()).map(str::to_string),
        notes: Vec::new(),
        transcript: None,
        cleaned_transcript: None,
        outline: None,
        highlights: None,
        missing_blocks: Vec::new(),
    };
    for n in v["note_list"].as_array().into_iter().flatten() {
        let markdown = n["data_content"].as_str().unwrap_or_default();
        if !markdown.trim().is_empty() {
            let tab = n["data_tab_name"].as_str().or(n["data_title"].as_str()).unwrap_or_default();
            d.notes.push(Note { tab: tab.to_string(), markdown: markdown.to_string() });
        }
    }
    for s in v["source_list"].as_array().into_iter().flatten() {
        let Some(block) = s["data_type"].as_str() else { continue };
        let raw = s["data_content"].as_str().unwrap_or_default();
        if raw.trim().is_empty() {
            // transaction_polish is often simply empty; only the transcript and
            // highlights are worth a second call.
            if block == "transaction" || block == "mark_memo" {
                d.missing_blocks.push(block.to_string());
            }
            continue;
        }
        if let Ok(content) = serde_json::from_str::<Value>(raw) {
            apply_block(&mut d, block, &content);
        }
    }
    Some(d)
}

fn apply_block(d: &mut Detail, block: &str, content: &Value) {
    match block {
        "transaction" => d.transcript = segments(content),
        "transaction_polish" => d.cleaned_transcript = segments(content),
        "outline" => d.outline = topics(content),
        "mark_memo" => d.highlights = non_empty(content),
        _ => {}
    }
}

fn non_empty(v: &Value) -> Option<Value> {
    match v {
        Value::Null => None,
        Value::Array(a) if a.is_empty() => None,
        Value::Object(o) if o.is_empty() => None,
        Value::String(s) if s.trim().is_empty() => None,
        other => Some(other.clone()),
    }
}

fn segments(v: &Value) -> Option<Vec<Segment>> {
    let out: Vec<Segment> = v
        .as_array()?
        .iter()
        .filter_map(|s| {
            let text = s["content"].as_str()?.trim().to_string();
            if text.is_empty() {
                return None;
            }
            let speaker = [&s["speaker"], &s["original_speaker"]]
                .into_iter()
                .find_map(|x| x.as_str().map(str::trim).filter(|x| !x.is_empty()))
                .map(str::to_string);
            Some(Segment { start_ms: s["start_time"].as_u64().unwrap_or(0), speaker, text })
        })
        .collect();
    (!out.is_empty()).then_some(out)
}

fn topics(v: &Value) -> Option<Vec<Topic>> {
    let out: Vec<Topic> = v
        .as_array()?
        .iter()
        .filter_map(|t| {
            let topic = t["topic"].as_str()?.trim().to_string();
            (!topic.is_empty()).then(|| Topic { start_ms: t["start_time"].as_u64().unwrap_or(0), topic })
        })
        .collect();
    (!out.is_empty()).then_some(out)
}

/// The transcript as text: one line per segment, as start time, speaker, then
/// text. SPEC.md "What is saved".
pub fn transcript_text(segments: &[Segment]) -> String {
    let mut out = String::new();
    for s in segments {
        out.push_str(&clock::offset(s.start_ms));
        out.push(' ');
        if let Some(sp) = &s.speaker {
            out.push_str(sp);
            out.push_str(": ");
        }
        out.push_str(&s.text.replace(['\r', '\n'], " "));
        out.push_str("\r\n");
    }
    out
}

/// The outline as text: one line per topic, as start time then topic.
pub fn outline_text(topics: &[Topic]) -> String {
    topics.iter().map(|t| format!("{} {}\r\n", clock::offset(t.start_ms), t.topic.replace(['\r', '\n'], " "))).collect()
}

/// The highlights as text. Their shape has not been seen yet (none of the
/// recordings tried had any), so each one is written as its time, if it has
/// one, then whatever text it carries; anything unrecognised is kept as JSON
/// rather than lost.
pub fn highlights_text(v: &Value) -> String {
    let items: Vec<&Value> = match v {
        Value::Array(a) => a.iter().collect(),
        Value::Object(o) => match o.get("marks").and_then(Value::as_array) {
            Some(a) => a.iter().collect(),
            None => vec![v],
        },
        other => vec![other],
    };
    let mut out = String::new();
    for item in items {
        let time = ["start_time", "time", "timestamp", "mark_time"].iter().find_map(|k| item[*k].as_u64());
        let text = ["content", "text", "note", "title", "topic"]
            .iter()
            .find_map(|k| item[*k].as_str().map(str::trim).filter(|s| !s.is_empty()));
        match (time, text) {
            (Some(t), Some(x)) => out.push_str(&format!("{} {}\r\n", clock::offset(t), x)),
            (Some(t), None) => out.push_str(&format!("{}\r\n", clock::offset(t))),
            (None, Some(x)) => out.push_str(&format!("{x}\r\n")),
            (None, None) => out.push_str(&format!("{item}\r\n")),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_file() -> Value {
        json!({
            "id": "of_test", "name": "Lecture 2026-09-24 09:00:26",
            "created_at": "2026-09-24T09:58:06", "start_at": "2026-09-24T08:00:26.699000",
            "duration": 7093000, "serial_number": "123", "presigned_url": "https://example.invalid/a?X-Amz-Signature=s",
            "note_list": [
                { "data_tab_name": "Summary", "data_title": "Summary", "data_type": "auto_sum_note", "data_content": "## Key points\n- One" },
                { "data_tab_name": "Empty", "data_type": "x", "data_content": "" }
            ],
            "source_list": [
                { "data_type": "transaction", "data_content": "[{\"start_time\":0,\"end_time\":11040,\"content\":\"Hello.\",\"speaker\":\"Speaker 1\",\"original_speaker\":\"Speaker 1\"},{\"start_time\":725000,\"end_time\":730000,\"content\":\"Next\\nline\",\"speaker\":\"\",\"original_speaker\":\"\"}]" },
                { "data_type": "transaction_polish", "data_content": "" },
                { "data_type": "outline", "data_content": "[{\"start_time\":0,\"end_time\":60000,\"topic\":\"Introduction\"}]" }
            ]
        })
    }

    #[test]
    fn get_file_is_read() {
        let d = parse_detail(&sample_file()).unwrap();
        assert_eq!(d.listed.id, "of_test");
        assert_eq!(d.listed.start.hour, 8);
        assert_eq!(d.listed.duration_ms, 7_093_000);
        assert!(d.audio_link.is_some());
        assert_eq!(d.notes, vec![Note { tab: "Summary".into(), markdown: "## Key points\n- One".into() }]);
        assert_eq!(d.transcript.as_ref().unwrap().len(), 2);
        assert!(d.cleaned_transcript.is_none(), "an empty block makes no file");
        assert!(d.missing_blocks.is_empty(), "an empty cleaned transcript is not fetched again");
        assert_eq!(d.outline.as_ref().unwrap()[0].topic, "Introduction");
        assert!(d.highlights.is_none());
    }

    #[test]
    fn debug_output_hides_the_audio_link() {
        let d = parse_detail(&sample_file()).unwrap();
        assert!(!format!("{d:?}").contains("Signature"));
    }

    #[test]
    fn an_empty_transcript_block_is_fetched_separately() {
        let mut f = sample_file();
        f["source_list"][0]["data_content"] = json!("");
        let d = parse_detail(&f).unwrap();
        assert!(d.transcript.is_none());
        assert_eq!(d.missing_blocks, vec!["transaction".to_string()]);
    }

    #[test]
    fn transcript_lines() {
        let d = parse_detail(&sample_file()).unwrap();
        assert_eq!(transcript_text(d.transcript.as_ref().unwrap()), "00:00 Speaker 1: Hello.\r\n12:05 Next line\r\n");
    }

    #[test]
    fn outline_lines() {
        let d = parse_detail(&sample_file()).unwrap();
        assert_eq!(outline_text(d.outline.as_ref().unwrap()), "00:00 Introduction\r\n");
    }

    #[test]
    fn highlights_of_unknown_shape_are_kept() {
        let v = json!([{ "start_time": 65000, "content": "Important" }, { "start_time": 70000 }, { "other": 1 }]);
        assert_eq!(highlights_text(&v), "01:05 Important\r\n01:10\r\n{\"other\":1}\r\n");
        let wrapped = json!({ "marks": [{ "time": 1000, "text": "x" }] });
        assert_eq!(highlights_text(&wrapped), "00:01 x\r\n");
    }

    #[test]
    fn listing_skips_items_without_an_id_or_time() {
        assert!(Listed::from_json(&json!({ "id": "", "start_at": "2026-09-24T08:00:26" })).is_none());
        assert!(Listed::from_json(&json!({ "id": "a" })).is_none());
        let l = Listed::from_json(&json!({ "id": "a", "created_at": "2026-09-24T08:00:26" })).unwrap();
        assert_eq!(l.start.hour, 8);
    }
}
