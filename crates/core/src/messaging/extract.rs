//! Turns the rows a terminal produced after a message into a clean answer (research 003 R5).

/// Drops the rows that echo the typed message and a trailing empty prompt.
pub fn clean_answer(rows: Vec<String>, message: &str, prompt: &str) -> String {
    let mut rows = rows;
    // The first rows hold the prompt plus the echoed message (possibly wrapped over lines).
    let msg_lines: Vec<&str> = message
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect();
    if let Some(last_msg_line) = msg_lines.last() {
        let tail: String = last_msg_line
            .chars()
            .rev()
            .take(40)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect();
        let scan = rows.len().min(msg_lines.len() * 4 + 4);
        if let Some(pos) = rows
            .iter()
            .take(scan)
            .position(|r| r.trim_end().ends_with(&tail))
        {
            rows.drain(..=pos);
        }
    }
    let prompt = prompt.trim_end();
    if !prompt.is_empty() && rows.last().is_some_and(|r| r.trim_end() == prompt) {
        rows.pop();
    }
    while rows.first().is_some_and(|r| r.trim().is_empty()) {
        rows.remove(0);
    }
    while rows.last().is_some_and(|r| r.trim().is_empty()) {
        rows.pop();
    }
    rows.join("\n")
}
