//! Browser nodes (feature 004): URL rules and the interface the desktop shell implements to drive
//! real pages. Core owns permissions and the model; the shell owns the webviews (research R8).

use std::path::Path;
use std::time::Duration;

use uuid::Uuid;

use crate::error::{AppError, AppResult, ErrorCode};

pub const DEFAULT_SIZE: (f64, f64) = (1024.0, 720.0);
pub const LOAD_TIMEOUT: Duration = Duration::from_secs(30);
pub const MAX_LOAD_TIMEOUT: Duration = Duration::from_secs(300);
pub const CLICK_TIMEOUT: Duration = Duration::from_secs(10);

/// Turns what the user typed into a URL the browser may load (research R10).
pub fn normalize_url(input: &str) -> AppResult<String> {
    let bad = |msg: &str| AppError::new(ErrorCode::InvalidAddress, msg);
    let t = input.trim();
    if t.is_empty() {
        return Err(bad("Enter an address."));
    }
    if t.chars().any(char::is_whitespace) {
        return Err(bad(
            "Enter an address, such as localhost:3000 or example.com.",
        ));
    }
    if t == "about:blank" {
        return Ok(t.to_string());
    }
    let lower = t.to_lowercase();
    if let Some((scheme, _)) = lower.split_once("://") {
        return match scheme {
            "http" | "https" => Ok(t.to_string()),
            _ => Err(bad("Only http and https addresses can open here.")),
        };
    }
    if lower.contains(':')
        && !lower.starts_with("localhost")
        && !lower.starts_with("127.0.0.1")
        && !lower.starts_with("[::1]")
    {
        // "mailto:x", "file:x" and similar, but not "host:port".
        let after = lower.split_once(':').map(|(_, r)| r).unwrap_or_default();
        if !after.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            return Err(bad("Only http and https addresses can open here."));
        }
    }
    let host = lower.split(['/', '?', '#']).next().unwrap_or_default();
    let host = host
        .rsplit_once(':')
        .filter(|(_, port)| port.chars().all(|c| c.is_ascii_digit()))
        .map(|(h, _)| h)
        .unwrap_or(host);
    let local = matches!(host, "localhost" | "127.0.0.1" | "[::1]");
    Ok(format!("{}://{t}", if local { "http" } else { "https" }))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PageInfo {
    pub url: String,
    pub title: String,
}

/// Page operations implemented by the shell with real webviews. Calls may block up to the given
/// timeout and are made from request threads, never from the UI thread.
pub trait BrowserHost: Send + Sync {
    fn open(&self, node: Uuid, url: &str, timeout: Duration) -> Result<PageInfo, String>;
    fn back(&self, node: Uuid, timeout: Duration) -> Result<PageInfo, String>;
    fn forward(&self, node: Uuid, timeout: Duration) -> Result<PageInfo, String>;
    fn reload(&self, node: Uuid, timeout: Duration) -> Result<PageInfo, String>;
    /// Title, URL, readable text, and numbered interactive elements, formatted for agents.
    fn read(&self, node: Uuid) -> Result<String, String>;
    fn click(&self, node: Uuid, target: &str, timeout: Duration) -> Result<PageInfo, String>;
    fn fill(&self, node: Uuid, target: &str, text: &str) -> Result<(), String>;
    fn press(&self, node: Uuid, key: &str) -> Result<(), String>;
    fn screenshot(&self, node: Uuid, out: &Path) -> Result<(), String>;
    /// Runs JavaScript and returns its result as JSON text.
    fn eval(&self, node: Uuid, js: &str) -> Result<String, String>;
    /// The node was deleted: free its page.
    fn close(&self, _node: Uuid) {}
    /// The workspace was deleted: close its pages and delete its browsing data (FR-010).
    fn forget_workspace(&self, _ws: Uuid, _nodes: &[Uuid]) {}
}

pub fn short_id(id: Uuid) -> String {
    format!("b-{}", &id.simple().to_string()[..4])
}
