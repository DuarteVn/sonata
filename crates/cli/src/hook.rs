//! End-of-turn hooks called by agents (research R5). They report to the app for the terminal
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

fn report(kind: &str, text: Option<String>) -> ExitCode {
    // Hooks must never break the agent: report quietly and always succeed.
    if let Ok(env) = client::env() {
        let _ = client::request(&env, &["_turn".to_string(), kind.to_string()], text, |_| {});
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
            let mut sink = String::new();
            let _ = std::io::stdin().read_to_string(&mut sink);
            report("start", None)
        }
        Some("codex-notify") => {
            let payload = args.get(1).cloned().unwrap_or_default();
            let v: Value = serde_json::from_str(&payload).unwrap_or(Value::Null);
            if v["type"] != "agent-turn-complete" {
                return ExitCode::SUCCESS;
            }
            report(
                "complete",
                v["last-assistant-message"].as_str().map(str::to_string),
            )
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
