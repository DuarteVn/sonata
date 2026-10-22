//! Turns the rows a terminal produced after a message into a clean answer (research 003 R5).

/// Drops the rows that echo the typed message and the prompt printed again at the end.
/// `prompt` holds the rows of the prompt as seen before sending, last row = input line.
pub fn clean_answer(rows: Vec<String>, message: &str, prompt: &[String]) -> String {
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
    // Remove the prompt from the end, bottom row first, as long as the rows match.
    for p in prompt.iter().rev() {
        while rows.last().is_some_and(|r| r.trim().is_empty()) && !p.trim().is_empty() {
            rows.pop();
        }
        match rows.last() {
            Some(r) if r.trim_end() == p.trim_end() => {
                rows.pop();
            }
            _ => break,
        }
    }
    while rows.first().is_some_and(|r| r.trim().is_empty()) {
        rows.remove(0);
    }
    while rows.last().is_some_and(|r| r.trim().is_empty()) {
        rows.pop();
    }
    rows.join("\n")
}

#[cfg(test)]
mod tests {
    use super::clean_answer;

    fn rows(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn strips_echo_and_single_line_prompt() {
        let r = rows(&["sh-3.2$ echo hi", "hi", "sh-3.2$"]);
        assert_eq!(
            clean_answer(r, "echo hi", &rows(&["", "", "sh-3.2$ "])),
            "hi"
        );
    }

    #[test]
    fn strips_two_line_prompt_but_keeps_matching_output() {
        let prompt = rows(&["", "/tmp", "❯ "]);
        let r = rows(&["❯ pwd", "/tmp", "", "/tmp", "❯"]);
        assert_eq!(clean_answer(r, "pwd", &prompt), "/tmp");
        let r = rows(&["❯ echo real-app-ok", "real-app-ok", "", "/tmp", "❯"]);
        assert_eq!(clean_answer(r, "echo real-app-ok", &prompt), "real-app-ok");
    }
}
