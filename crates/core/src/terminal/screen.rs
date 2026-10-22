//! Headless terminal screen model (research R1). Every PTY byte goes through it, so screens,
//! previews, and history exist without any view attached.

use std::num::NonZeroU16;

use vt100::capture::{BasicFormattedCaptureRange, BasicFormattedCaptureState};

/// A terminal emulator state that can be rebuilt elsewhere from its own replay.
/// Implemented with `vt100`; kept as a trait so the emulator can be swapped (plan.md,
/// Complexity Tracking).
pub trait ScreenModel: Send {
    fn process(&mut self, bytes: &[u8]);
    fn resize(&mut self, rows: u16, cols: u16);
    fn size(&self) -> (u16, u16);
    /// Bytes that, fed into a fresh terminal of the same size, reproduce the scrollback, the
    /// visible screen, the cursor, and the input modes.
    fn replay(&self) -> Vec<u8>;
    /// Plain text of the visible rows.
    fn preview_lines(&self) -> Vec<String>;
    fn cursor(&self) -> (u16, u16);
    /// Scrollback plus visible rows, oldest first, with SGR colors, at most `max_rows` rows.
    fn history_rows(&self, max_rows: usize) -> Vec<String>;
    fn alternate_screen(&self) -> bool;
}

pub struct Vt100Screen {
    parser: vt100::Parser,
}

fn nz(v: u16) -> NonZeroU16 {
    NonZeroU16::new(v.max(1)).expect("non-zero")
}

impl Vt100Screen {
    pub fn new(rows: u16, cols: u16, scrollback: usize) -> Self {
        Self {
            parser: vt100::Parser::new(nz(rows), nz(cols), scrollback),
        }
    }

    pub fn screen(&self) -> &vt100::Screen {
        self.parser.screen()
    }

    /// Scrollback and visible rows of the active grid, oldest first, SGR only, `\n` between
    /// rows that do not wrap.
    fn full_capture(&self) -> String {
        let mut out = String::new();
        let mut state = BasicFormattedCaptureState::default();
        let _ = self
            .parser
            .screen()
            .write_contents_formatted_basic(&mut out, BasicFormattedCaptureRange::Full(&mut state));
        out
    }
}

impl ScreenModel for Vt100Screen {
    fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    fn resize(&mut self, rows: u16, cols: u16) {
        self.parser.screen_mut().set_size(nz(rows), nz(cols));
    }

    fn size(&self) -> (u16, u16) {
        let (rows, cols) = self.parser.screen().size();
        (rows.get(), cols.get())
    }

    fn replay(&self) -> Vec<u8> {
        let screen = self.parser.screen();
        let mut out = String::from("\x1b[0m\x1b[H\x1b[2J");
        // 1. Every row, oldest first. Rows beyond the screen height scroll into the receiving
        //    terminal's scrollback, reproducing ours.
        if !screen.alternate_screen() {
            out.push_str(&self.full_capture().replace('\n', "\r\n"));
            out.push_str("\x1b[0m");
        }
        // 2. Redraw the visible screen exactly (clears it first; clearing does not touch
        //    scrollback), then restore cursor and input modes.
        if screen.alternate_screen() {
            out.push_str("\x1b[?1049h");
        }
        out.push_str(&screen.state_formatted());
        out.into_bytes()
    }

    fn preview_lines(&self) -> Vec<String> {
        let screen = self.parser.screen();
        let (_, cols) = screen.size();
        screen.rows(0, cols.get()).collect()
    }

    fn cursor(&self) -> (u16, u16) {
        self.parser.screen().cursor_position()
    }

    fn history_rows(&self, max_rows: usize) -> Vec<String> {
        let capture = self.full_capture();
        let rows: Vec<&str> = capture.split('\n').collect();
        let skip = rows.len().saturating_sub(max_rows);
        let mut kept: Vec<String> = rows[skip..].iter().map(|r| (*r).to_string()).collect();
        // The capture only emits SGR changes, so attributes set in a dropped row may still
        // apply. Carry them into the first kept row.
        let carried = active_sgr(&rows[..skip]);
        if let Some(first) = kept.first_mut() {
            first.insert_str(0, &carried);
        }
        kept
    }

    fn alternate_screen(&self) -> bool {
        self.parser.screen().alternate_screen()
    }
}

/// SGR sequences still in effect after `rows`: everything since the last full reset.
fn active_sgr(rows: &[&str]) -> String {
    let mut active = String::new();
    for row in rows {
        let mut rest = *row;
        while let Some(start) = rest.find("\x1b[") {
            let after = &rest[start + 2..];
            let Some(end) = after.find(|c: char| c.is_ascii_alphabetic()) else {
                break;
            };
            if after.as_bytes()[end] == b'm' {
                let params = &after[..end];
                if params.is_empty() || params == "0" {
                    active.clear();
                } else if let Some(stripped) = params.strip_prefix("0;") {
                    active.clear();
                    active.push_str(&format!("\x1b[{stripped}m"));
                } else {
                    active.push_str(&rest[start..start + 2 + end + 1]);
                }
            }
            rest = &after[end + 1..];
        }
    }
    active
}
