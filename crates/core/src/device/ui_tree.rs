//! uiautomator XML → numbered elements for `sonata device read` (research R4, FR-012).

use std::collections::HashMap;

use quick_xml::events::{BytesStart, Event};

/// Most bytes `read` prints (FR-012).
pub const MAX_READ_BYTES: usize = 100 * 1024;
const MAX_LABEL_CHARS: usize = 120;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Element {
    pub number: usize,
    pub kind: String,
    pub label: String,
    /// The part of `resource-id` after `:id/`.
    pub resource_id: Option<String>,
    /// `(x1, y1, x2, y2)` in device pixels.
    pub bounds: (i32, i32, i32, i32),
    pub tappable: bool,
    pub editable: bool,
    pub scrollable: bool,
    pub password: bool,
    pub focused: bool,
}

impl Element {
    pub fn center(&self) -> (i32, i32) {
        let (x1, y1, x2, y2) = self.bounds;
        ((x1 + x2) / 2, (y1 + y2) / 2)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Screen {
    /// Package of the foreground app.
    pub app: String,
    pub size: (u32, u32),
    pub elements: Vec<Element>,
}

#[derive(Default)]
struct Raw {
    attrs: HashMap<String, String>,
    children: Vec<Raw>,
}

impl Raw {
    fn get(&self, key: &str) -> &str {
        self.attrs.get(key).map(String::as_str).unwrap_or_default()
    }

    fn flag(&self, key: &str) -> bool {
        self.get(key) == "true"
    }

    fn bounds(&self) -> Option<(i32, i32, i32, i32)> {
        // "[x1,y1][x2,y2]"
        let nums: Vec<i32> = self
            .get("bounds")
            .split(|c: char| !(c.is_ascii_digit() || c == '-'))
            .filter(|s| !s.is_empty())
            .filter_map(|s| s.parse().ok())
            .collect();
        match nums[..] {
            [x1, y1, x2, y2] => Some((x1, y1, x2, y2)),
            _ => None,
        }
    }

    fn class_name(&self) -> &str {
        self.get("class").rsplit('.').next().unwrap_or_default()
    }

    fn editable(&self) -> bool {
        let c = self.class_name();
        c.ends_with("EditText") || c.ends_with("AutoCompleteTextView")
    }

    fn tappable(&self) -> bool {
        self.flag("clickable") || self.flag("long-clickable") || self.flag("checkable")
    }

    /// Its own words: text (never a password's) or description.
    fn own_text(&self) -> Option<String> {
        let text = if self.flag("password") {
            ""
        } else {
            self.get("text")
        };
        [text, self.get("content-desc")]
            .into_iter()
            .map(clean)
            .find(|t| !t.is_empty())
    }

    fn resource_id(&self) -> Option<String> {
        let id = self.get("resource-id");
        let tail = id.rsplit_once(":id/").map_or(id, |(_, t)| t);
        (!tail.is_empty()).then(|| tail.to_string())
    }

    fn kind(&self) -> &'static str {
        let c = self.class_name();
        if self.editable() {
            "input"
        } else if c.contains("CheckBox") {
            "checkbox"
        } else if c.contains("RadioButton") {
            "radio"
        } else if c.contains("Switch") || c.contains("ToggleButton") {
            "switch"
        } else if c.contains("Button") {
            "button"
        } else if self.flag("scrollable") {
            "list"
        } else if c.contains("ImageView") {
            "image"
        } else if c.contains("TextView") {
            "text"
        } else {
            "view"
        }
    }
}

fn clean(s: &str) -> String {
    let joined = s
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .replace('"', "'");
    if joined.chars().count() > MAX_LABEL_CHARS {
        joined.chars().take(MAX_LABEL_CHARS - 1).collect::<String>() + "…"
    } else {
        joined
    }
}

fn start(e: &BytesStart<'_>) -> Result<Raw, String> {
    let mut raw = Raw::default();
    for a in e.attributes() {
        let a = a.map_err(|e| e.to_string())?;
        let key = a.key.as_ref().to_string();
        let value = a
            .normalized_value(quick_xml::XmlVersion::Implicit1_0)
            .map_err(|e| e.to_string())?
            .into_owned();
        raw.attrs.insert(key, value);
    }
    Ok(raw)
}

fn parse_tree(xml: &str) -> Result<Raw, String> {
    let xml = super::adb::strip_dump_status(xml);
    if !xml.contains("<hierarchy") {
        return Err(format!(
            "the device returned no screen tree: {}",
            xml.lines().next().unwrap_or_default().trim()
        ));
    }
    let mut reader = quick_xml::Reader::from_str(xml);
    let mut stack: Vec<Raw> = vec![Raw::default()];
    loop {
        match reader
            .read_event()
            .map_err(|e| format!("bad screen tree: {e}"))?
        {
            Event::Start(e) if e.name().as_ref() == "node" => stack.push(start(&e)?),
            Event::Empty(e) if e.name().as_ref() == "node" => {
                let node = start(&e)?;
                stack.last_mut().expect("root").children.push(node);
            }
            Event::End(e) if e.name().as_ref() == "node" => {
                let node = stack.pop().ok_or("bad screen tree")?;
                stack
                    .last_mut()
                    .ok_or("bad screen tree")?
                    .children
                    .push(node);
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if stack.len() != 1 {
        return Err("bad screen tree: unclosed elements".into());
    }
    Ok(stack.pop().expect("root"))
}

/// Texts of a tappable element's descendants, in order (they become its label).
fn child_texts(raw: &Raw, out: &mut Vec<String>) {
    for c in &raw.children {
        if let Some(t) = c.own_text() {
            out.push(t);
        }
        child_texts(c, out);
    }
}

fn visible(b: (i32, i32, i32, i32), (w, h): (u32, u32)) -> bool {
    let (x1, y1, x2, y2) = b;
    x2 > x1 && y2 > y1 && x2 > 0 && y2 > 0 && x1 < w as i32 && y1 < h as i32
}

fn walk(raw: &Raw, size: (u32, u32), in_tappable: bool, out: &mut Vec<Element>) {
    let mut inside = in_tappable;
    if let Some(bounds) = raw.bounds().filter(|b| visible(*b, size)) {
        let (tappable, editable, scrollable) =
            (raw.tappable(), raw.editable(), raw.flag("scrollable"));
        let own = raw.own_text();
        let numbered = tappable || editable || scrollable || (own.is_some() && !in_tappable);
        if numbered {
            let label = if editable {
                own.or_else(|| Some(clean(raw.get("hint"))).filter(|h| !h.is_empty()))
            } else {
                own.or_else(|| {
                    if !tappable {
                        return None;
                    }
                    let mut texts = Vec::new();
                    child_texts(raw, &mut texts);
                    (!texts.is_empty()).then(|| clean(&texts.join(" · ")))
                })
            }
            .or_else(|| raw.resource_id())
            .unwrap_or_default();
            out.push(Element {
                number: out.len() + 1,
                kind: raw.kind().to_string(),
                label,
                resource_id: raw.resource_id(),
                bounds,
                tappable,
                editable,
                scrollable,
                password: raw.flag("password"),
                focused: raw.flag("focused"),
            });
            inside = in_tappable || tappable;
        }
    }
    for c in &raw.children {
        walk(c, size, inside, out);
    }
}

pub fn parse_dump(xml: &str) -> Result<Screen, String> {
    let root = parse_tree(xml)?;
    let first = root.children.first();
    let size = first
        .and_then(Raw::bounds)
        .map(|(_, _, x2, y2)| (x2.max(0) as u32, y2.max(0) as u32))
        .unwrap_or((0, 0));
    let mut elements = Vec::new();
    for c in &root.children {
        walk(c, size, false, &mut elements);
    }
    Ok(Screen {
        app: first
            .map(|f| f.get("package").to_string())
            .unwrap_or_default(),
        size,
        elements,
    })
}

fn line(e: &Element) -> String {
    let (x1, y1, x2, y2) = e.bounds;
    let mut s = format!("  [{}] {:<8} \"{}\"", e.number, e.kind, e.label);
    if let Some(id) = &e.resource_id {
        s.push_str(&format!("  #{id}"));
    }
    s.push_str(&format!("  [{x1},{y1}][{x2},{y2}]"));
    let flags: Vec<&str> = [
        (e.tappable, "tappable"),
        (e.editable, "editable"),
        (e.scrollable, "scrollable"),
        (e.password, "password"),
        (e.focused, "focused"),
    ]
    .into_iter()
    .filter_map(|(on, name)| on.then_some(name))
    .collect();
    if !flags.is_empty() {
        s.push_str("  ");
        s.push_str(&flags.join(", "));
    }
    s
}

/// The text `sonata device read` prints, at most 100 KB.
pub fn format_read(screen: &Screen) -> String {
    let mut out = format!(
        "App: {}\nScreen: {}x{}\n\n",
        screen.app, screen.size.0, screen.size.1
    );
    if screen.elements.is_empty() {
        out.push_str(
            "No elements on screen (for example a game or a video). Coordinate taps and screenshots still work.",
        );
        return out;
    }
    let reserve = 80;
    for (i, e) in screen.elements.iter().enumerate() {
        let l = line(e);
        if out.len() + l.len() + 1 > MAX_READ_BYTES - reserve {
            out.push_str(&format!(
                "  … {} more elements; scroll or use coordinates",
                screen.elements.len() - i
            ));
            return out;
        }
        out.push_str(&l);
        out.push('\n');
    }
    out.truncate(out.trim_end().len());
    out
}
