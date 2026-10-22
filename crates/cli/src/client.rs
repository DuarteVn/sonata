//! Socket client: one JSON request line, then progress lines and one result line.

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixStream;
use std::process::ExitCode;

use serde_json::{Value, json};

use crate::EXIT_UNREACHABLE;

pub struct Env {
    pub socket: String,
    pub terminal_id: String,
    pub token: String,
}

/// Reads the terminal's Sonata identity, or explains that `sonata` only works inside Sonata.
pub fn env() -> Result<Env, ExitCode> {
    let get = |k: &str| std::env::var(k).ok().filter(|v| !v.is_empty());
    match (
        get("SONATA_SOCKET"),
        get("SONATA_TERMINAL_ID"),
        get("SONATA_TOKEN"),
    ) {
        (Some(socket), Some(terminal_id), Some(token)) => Ok(Env {
            socket,
            terminal_id,
            token,
        }),
        _ => {
            eprintln!("sonata: this command only works inside a Sonata terminal.");
            Err(ExitCode::from(EXIT_UNREACHABLE))
        }
    }
}

/// Sends a request and returns the final result object.
pub fn request(
    env: &Env,
    args: &[String],
    stdin: Option<String>,
    mut on_progress: impl FnMut(&str),
) -> Result<Value, String> {
    let mut stream = UnixStream::connect(&env.socket)
        .map_err(|e| format!("Sonata is not reachable ({e}). Is the app running?"))?;
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_default();
    let req = json!({"v": 1, "terminalId": env.terminal_id, "token": env.token, "cwd": cwd, "args": args, "stdin": stdin});
    let mut line = serde_json::to_vec(&req).map_err(|e| e.to_string())?;
    line.push(b'\n');
    stream
        .write_all(&line)
        .map_err(|e| format!("Sonata is not reachable ({e})."))?;
    for line in BufReader::new(stream).lines() {
        let line = line.map_err(|e| format!("Connection to Sonata broke ({e})."))?;
        let msg: Value =
            serde_json::from_str(&line).map_err(|e| format!("Bad reply from Sonata ({e})."))?;
        match msg["type"].as_str() {
            Some("progress") => on_progress(msg["message"].as_str().unwrap_or_default()),
            Some("result") => return Ok(msg),
            _ => {}
        }
    }
    Err("Sonata closed the connection without an answer.".into())
}

pub fn run(args: Vec<String>, stdin: Option<String>, json_out: bool) -> ExitCode {
    let env = match env() {
        Ok(e) => e,
        Err(code) => return code,
    };
    match request(&env, &args, stdin, |p| eprintln!("sonata: {p}")) {
        Ok(result) => {
            let code = result["exitCode"].as_u64().unwrap_or(1) as u8;
            if json_out {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&result).unwrap_or_default()
                );
            } else if let Some(text) = result["text"].as_str().filter(|t| !t.is_empty()) {
                // A timed-out ask still carries the partial answer: print it like a result.
                if code == 0 || code == 5 {
                    println!("{text}");
                } else {
                    eprintln!("sonata: {text}");
                }
            }
            ExitCode::from(code)
        }
        Err(e) => {
            eprintln!("sonata: {e}");
            ExitCode::from(EXIT_UNREACHABLE)
        }
    }
}
