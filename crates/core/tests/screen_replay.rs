//! R1 spike: a screen rebuilt from `replay()` must match the original.

use pretty_assertions::assert_eq;
use sonata_core::terminal::screen::{ScreenModel, Vt100Screen};

const ROWS: u16 = 30;
const COLS: u16 = 100;

fn snapshot(s: &Vt100Screen) -> Vec<String> {
    let screen = s.screen();
    let mut out = vec![format!(
        "cursor={:?} alt={}",
        screen.cursor_position(),
        screen.alternate_screen()
    )];
    for row in 0..ROWS {
        let mut line = String::new();
        for col in 0..COLS {
            let Some(cell) = screen.cell(row, col) else {
                continue;
            };
            line.push_str(&format!(
                "[{}|{:?}|{:?}|{}{}{}{}]",
                cell.contents(),
                cell.fgcolor(),
                cell.bgcolor(),
                u8::from(cell.bold()),
                u8::from(cell.italic()),
                u8::from(cell.underline()),
                u8::from(cell.inverse()),
            ));
        }
        out.push(line);
    }
    out
}

fn scrollback_text(s: &Vt100Screen) -> Vec<String> {
    s.history_rows(usize::MAX)
}

fn assert_replays(bytes: &[u8]) {
    let mut original = Vt100Screen::new(ROWS, COLS, 10_000);
    original.process(bytes);
    let mut rebuilt = Vt100Screen::new(ROWS, COLS, 10_000);
    rebuilt.process(&original.replay());
    assert_eq!(snapshot(&rebuilt), snapshot(&original));
    if !original.alternate_screen() {
        assert_eq!(scrollback_text(&rebuilt), scrollback_text(&original));
    }
}

#[test]
fn vim_fixture() {
    assert_replays(include_bytes!("fixtures/vim.bin"));
}

#[test]
fn top_fixture() {
    assert_replays(include_bytes!("fixtures/top.bin"));
}

#[test]
fn long_colored_output_with_scrollback() {
    let mut bytes = Vec::new();
    for i in 0..500 {
        bytes.extend(
            format!(
                "\x1b[3{}mline {i}\x1b[0m \x1b[1mbold\x1b[0m 日本語 ✓\r\n",
                i % 8
            )
            .as_bytes(),
        );
    }
    bytes.extend(b"$ ");
    assert_replays(&bytes);
}

#[test]
fn wrapped_lines() {
    let long = "x".repeat(250);
    assert_replays(format!("{long}\r\nshort\r\n{long}").as_bytes());
}

/// Agent TUIs (React/ink style) redraw a block in place: cursor up, erase line, rewrite.
#[test]
fn in_place_redraws_like_agent_tuis() {
    let mut bytes = Vec::new();
    for i in 0..40 {
        bytes.extend(format!("history {i}\r\n").as_bytes());
    }
    bytes.extend(b"\x1b[?25l");
    for frame in ["⠋", "⠙", "⠹", "⠸"] {
        bytes.extend(format!("\x1b[2K\r{frame} Thinking…\r\n\x1b[2K\r> \x1b[1A").as_bytes());
    }
    bytes.extend("\x1b[2K\r\x1b[32m●\x1b[0m Done\r\n\x1b[2K\r> type here".as_bytes());
    assert_replays(&bytes);
}

#[test]
fn alternate_screen_program() {
    let mut bytes = b"before\r\n".to_vec();
    bytes.extend(b"\x1b[?1049h\x1b[H\x1b[2J\x1b[7m status bar \x1b[0m\x1b[10;5Hcontent\x1b[?1h");
    assert_replays(&bytes);
}

#[test]
fn history_is_capped_and_preview_is_plain_text() {
    let mut s = Vt100Screen::new(5, 20, 10_000);
    for i in 0..50 {
        s.process(format!("\x1b[31mrow {i}\x1b[0m\r\n").as_bytes());
    }
    let rows = s.history_rows(10);
    assert_eq!(rows.len(), 10);
    assert!(
        rows.iter().any(|r| r.contains("\x1b[31m")),
        "colors kept: {rows:?}"
    );
    let preview = s.preview_lines();
    assert_eq!(preview.len(), 5);
    assert_eq!(preview[0], "row 46");
}
