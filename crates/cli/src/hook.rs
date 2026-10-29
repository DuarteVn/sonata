//! End-of-turn and conversation hooks called by agents (research R5; feature 009). They report to the app for the terminal
//! they run in; agents never call them directly.

use std::io::{BufRead, BufReader, Read};
use std::process::ExitCode;

use serde_json::Value;

use crate::client;

/// Text of the assistant's final message: assistant text blocks after the last user entry.
pub fn last_assistant_text(transcript: &str) -> String {
    let mut parts: Vec<String> = Vec::new();
    for line in transcript.lines() {
        let Ok(v) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        match v["type"].as_str() {
            Some("user") => parts.clear(),
            Some("assistant") => {
                if let Some(blocks) = v["message"]["content"].as_array() {
                    for b in blocks {
                        if b["type"] == "text"
                            && let Some(t) = b["text"].as_str()
                        {
                            parts.push(t.to_string());
                        }
                    }
                }
            }
            _ => {}
        }
    }
    parts.join("\n\n")
}

/// Sends one request quietly: hooks must never break the agent.
fn send(args: &[&str], text: Option<String>) {
    if let Ok(env) = client::env() {
        let args: Vec<String> = args.iter().map(|a| a.to_string()).collect();
        let _ = client::request(&env, &args, text, |_| {});
    }
}

fn report(kind: &str, text: Option<String>) -> ExitCode {
    send(&["_turn", kind], text);
    ExitCode::SUCCESS
}

/// Reports the conversation the agent is in (`None`: a new, empty one) so the terminal can
/// resume it later (feature 009). Print-mode runs set `SONATA_CONVERSATION=off`.
fn report_conversation(agent: &str, id: Option<&str>) {
    if std::env::var("SONATA_CONVERSATION").is_ok_and(|v| v == "off") {
        return;
    }
    send(&["_conversation", agent, id.unwrap_or("none")], None);
}

fn stdin_json() -> Value {
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);
    serde_json::from_str(&input).unwrap_or(Value::Null)
}

/// Kimi Code hooks (feature 010, contracts/cli.md). Kimi adds `UserPromptSubmit` output to the
/// model's context, so these print nothing; a bad payload reports nothing.
fn kimi(event: Option<&str>) -> ExitCode {
    let v = stdin_json();
    if v.is_null() {
        return ExitCode::SUCCESS;
    }
    let session = v["session_id"].as_str();
    match event {
        Some("prompt-submit") => {
            report("start", None);
            if let Some(id) = session {
                report_conversation("kimi", Some(id));
            }
        }
        Some("stop" | "stop-failure" | "interrupt") => {
            report("complete", None);
        }
        // `resume`: an existing conversation; `startup`: a new, empty one (research R6).
        Some("session-start") => match (v["source"].as_str(), session) {
            (Some("resume"), Some(id)) => report_conversation("kimi", Some(id)),
            (Some("startup"), _) => report_conversation("kimi", None),
            _ => {}
        },
        _ => {}
    }
    ExitCode::SUCCESS
}

pub fn main(args: Vec<String>) -> ExitCode {
    match args.first().map(String::as_str) {
        Some("claude-stop") => {
            let mut input = String::new();
            let _ = std::io::stdin().read_to_string(&mut input);
            let v: Value = serde_json::from_str(&input).unwrap_or(Value::Null);
            let text = v["last_assistant_message"]
                .as_str()
                .map(str::to_string)
                .or_else(|| {
                    let path = v["transcript_path"].as_str()?;
                    let file = std::fs::File::open(path).ok()?;
                    let mut s = String::new();
                    for line in BufReader::new(file).lines().map_while(Result::ok) {
                        s.push_str(&line);
                        s.push('\n');
                    }
                    Some(last_assistant_text(&s))
                });
            report("complete", text)
        }
        Some("turn-start") => {
            let v = stdin_json();
            report("start", None);
            if let Some(id) = v["session_id"].as_str() {
                report_conversation("claude", Some(id));
            }
            ExitCode::SUCCESS
        }
        Some("session-start") => {
            // `resume`: a conversation with messages; `startup`/`clear`: a new, empty one.
            let v = stdin_json();
            match (v["source"].as_str(), v["session_id"].as_str()) {
                (Some("resume"), Some(id)) => report_conversation("claude", Some(id)),
                (Some("startup" | "clear"), _) => report_conversation("claude", None),
                _ => {}
            }
            ExitCode::SUCCESS
        }
        Some("kimi") => kimi(args.get(1).map(String::as_str)),
        Some("codex-notify") => {
            let payload = args.get(1).cloned().unwrap_or_default();
            let v: Value = serde_json::from_str(&payload).unwrap_or(Value::Null);
            if v["type"] != "agent-turn-complete" {
                return ExitCode::SUCCESS;
            }
            report(
                "complete",
                v["last-assistant-message"].as_str().map(str::to_string),
            );
            if let Some(id) = v["thread-id"].as_str() {
                report_conversation("codex", Some(id));
            }
            ExitCode::SUCCESS
        }
        _ => ExitCode::SUCCESS,
    }
}

#[cfg(test)]
mod tests {
    use super::last_assistant_text;

    #[test]
    fn takes_text_after_last_user_entry() {
        let t = r#"{"type":"user","message":{"content":"hi"}}
{"type":"assistant","message":{"content":[{"type":"text","text":"old"}]}}
{"type":"user","message":{"content":[{"type":"tool_result"}]}}
{"type":"assistant","message":{"content":[{"type":"tool_use"},{"type":"text","text":"Final"}]}}
{"type":"assistant","message":{"content":[{"type":"text","text":"answer"}]}}"#;
        assert_eq!(last_assistant_text(t), "Final\n\nanswer");
    }
}
