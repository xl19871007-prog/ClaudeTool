use crate::error::Result;
use crate::fs as appfs;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionMeta {
    pub id: String,
    pub name: Option<String>,
    pub summary: Option<String>,
    pub first_prompt: String,
    pub cwd: String,
    pub turn_count: u32,
    /// ISO 8601 string from the first user line's `timestamp`.
    pub created_at: Option<String>,
    /// Unix epoch seconds (UTC) from the file mtime. Frontend converts to Date.
    pub updated_at_unix: u64,
    pub bytes: u64,
}

/// Encode a workdir path the way Claude Code stores it under
/// `~/.claude/projects/<encoded>/`. Empirically: every `:`, `\`, `/`
/// is replaced with `-`. Consecutive separators yield consecutive dashes.
pub fn encode_cwd(workdir: &Path) -> String {
    workdir
        .to_string_lossy()
        .chars()
        .map(|c| if c == '\\' || c == '/' || c == ':' { '-' } else { c })
        .collect()
}

fn project_dir(workdir: &Path) -> Option<PathBuf> {
    appfs::claude_home().map(|h| h.join("projects").join(encode_cwd(workdir)))
}

#[derive(Debug, Deserialize)]
struct UserLine {
    #[serde(rename = "type")]
    line_type: String,
    message: Option<UserMessage>,
    timestamp: Option<String>,
    cwd: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserMessage {
    role: Option<String>,
    content: Option<serde_json::Value>,
}

fn extract_text(content: &serde_json::Value) -> String {
    match content {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => arr
            .iter()
            .filter_map(|item| {
                item.get("text")
                    .and_then(|t| t.as_str())
                    .map(String::from)
            })
            .collect::<Vec<_>>()
            .join(" "),
        _ => String::new(),
    }
}

fn truncate(s: &str, max_chars: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max_chars {
        s.to_string()
    } else {
        let mut out: String = chars.into_iter().take(max_chars).collect();
        out.push('…');
        out
    }
}

/// Streaming parse: read lines until we have first user prompt + cwd, then stop.
/// Returns `None` if file is malformed / has no user message.
fn parse_session_quick(path: &Path) -> Option<SessionMeta> {
    let file = File::open(path).ok()?;
    let metadata = file.metadata().ok()?;
    let bytes = metadata.len();
    let updated_at_unix = metadata
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let id = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();

    let reader = BufReader::new(file);
    let mut first_prompt: Option<String> = None;
    let mut cwd: Option<String> = None;
    let mut created_at: Option<String> = None;

    // Scan up to first user message; cap at 200 lines to bound work on weird files.
    for line in reader.lines().take(200) {
        let Ok(line) = line else { continue };
        if line.trim().is_empty() {
            continue;
        }
        let Ok(parsed) = serde_json::from_str::<UserLine>(&line) else {
            continue;
        };
        if parsed.line_type != "user" {
            continue;
        }
        let Some(msg) = parsed.message else { continue };
        if msg.role.as_deref() != Some("user") {
            continue;
        }
        let Some(content) = msg.content else { continue };
        let text = extract_text(&content);
        if text.is_empty() {
            continue;
        }
        first_prompt = Some(truncate(&text, 200));
        cwd = parsed.cwd;
        created_at = parsed.timestamp;
        break;
    }

    let first_prompt = first_prompt?;
    let cwd = cwd.unwrap_or_default();

    Some(SessionMeta {
        id,
        name: None,
        summary: None,
        first_prompt,
        cwd,
        turn_count: 0, // refined later
        created_at,
        updated_at_unix,
        bytes,
    })
}

pub fn list_sessions_quick(workdir: &Path) -> Result<Vec<SessionMeta>> {
    let Some(dir) = project_dir(workdir) else {
        return Ok(Vec::new());
    };
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut sessions: Vec<(SessionMeta, std::time::SystemTime)> = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let Ok(entry) = entry else { continue };
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) != Some("jsonl") {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        let Some(session) = parse_session_quick(&path) else {
            continue;
        };
        let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
        sessions.push((session, mtime));
    }

    sessions.sort_by(|a, b| b.1.cmp(&a.1));
    Ok(sessions.into_iter().map(|(s, _)| s).collect())
}

/// Refines `turn_count` for one session by counting all `type: "user"` lines.
/// Designed to be called per-session in a background task and pushed via event.
pub fn refine_turn_count(workdir: &Path, session_id: &str) -> Option<u32> {
    let dir = project_dir(workdir)?;
    let path = dir.join(format!("{session_id}.jsonl"));
    let file = File::open(path).ok()?;
    let reader = BufReader::new(file);
    let mut count: u32 = 0;
    for line in reader.lines() {
        let Ok(line) = line else { continue };
        if line.trim().is_empty() {
            continue;
        }
        // Cheap check before full JSON parse
        if !line.contains("\"type\":\"user\"") {
            continue;
        }
        let Ok(parsed) = serde_json::from_str::<UserLine>(&line) else {
            continue;
        };
        if parsed.line_type == "user"
            && parsed.message.and_then(|m| m.role).as_deref() == Some("user")
        {
            count += 1;
        }
    }
    Some(count)
}
