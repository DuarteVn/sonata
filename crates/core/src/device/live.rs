//! The live screen of a device node: latest-frame-only delivery with UI acknowledgment, the card
//! image, and input from the node (research R2, R3).

use std::path::Path;
use std::sync::{Arc, Mutex};

use serde::Deserialize;

use super::grpc::Frame;
use super::{Key, StreamHandle};

/// Bytes before the RGBA pixels: width, height, sequence number, each a little-endian u32.
pub const FRAME_HEADER: usize = 12;

pub fn encode_frame(f: &Frame) -> Vec<u8> {
    let mut out = Vec::with_capacity(FRAME_HEADER + f.rgba.len());
    for v in [f.width, f.height, f.seq] {
        out.extend_from_slice(&v.to_le_bytes());
    }
    out.extend_from_slice(&f.rgba);
    out
}

/// `(width, height, seq)` of an encoded frame.
pub fn decode_header(b: &[u8]) -> Option<(u32, u32, u32)> {
    let word = |i: usize| Some(u32::from_le_bytes(b.get(i..i + 4)?.try_into().ok()?));
    Some((word(0)?, word(4)?, word(8)?))
}

/// What the node sends from mouse and keyboard, in device pixels (contracts/ipc.md).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LiveInput {
    Touch {
        x: i32,
        y: i32,
        down: bool,
    },
    Wheel {
        x: i32,
        y: i32,
        dx: i32,
        dy: i32,
    },
    Text(String),
    /// `back`, `home`, `recents` from the node's buttons, or a W3C key name from the keyboard.
    Key(String),
}

impl LiveInput {
    pub fn key(name: &str) -> Key {
        match name {
            "back" => Key::Back,
            "home" => Key::Home,
            "recents" => Key::Recents,
            other => Key::Named(other.to_string()),
        }
    }
}

pub type FrameSink = Box<dyn Fn(Vec<u8>) + Send + Sync>;
/// The newest frame of a node, shared with its stream.
pub type LastFrame = Arc<Mutex<Option<Arc<Frame>>>>;

struct Gate {
    send: FrameSink,
    in_flight: bool,
    pending: Option<Arc<Frame>>,
}

/// One open stream from a device to its node.
pub struct Live {
    gate: Arc<Mutex<Gate>>,
    last: LastFrame,
    _handle: StreamHandle,
}

impl Live {
    /// Opens the stream through `open`, which gets the frame callback.
    pub fn start(
        send: FrameSink,
        last: LastFrame,
        open: impl FnOnce(Box<dyn FnMut(Frame) + Send>) -> Result<StreamHandle, String>,
    ) -> Result<Self, String> {
        let gate = Arc::new(Mutex::new(Gate {
            send,
            in_flight: false,
            pending: None,
        }));
        let (g, l) = (gate.clone(), last.clone());
        let handle = open(Box::new(move |frame| {
            let frame = Arc::new(frame);
            *l.lock().unwrap_or_else(|e| e.into_inner()) = Some(frame.clone());
            let mut gate = g.lock().unwrap_or_else(|e| e.into_inner());
            if gate.in_flight {
                gate.pending = Some(frame);
            } else {
                gate.in_flight = true;
                (gate.send)(encode_frame(&frame));
            }
        }))?;
        Ok(Self {
            gate,
            last,
            _handle: handle,
        })
    }

    /// The UI drew the previous frame: send the newest one waiting, if any.
    pub fn ack(&self) {
        let mut gate = self.gate.lock().unwrap_or_else(|e| e.into_inner());
        gate.in_flight = false;
        if let Some(f) = gate.pending.take() {
            gate.in_flight = true;
            (gate.send)(encode_frame(&f));
        }
    }

    pub fn last(&self) -> Option<Arc<Frame>> {
        self.last.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }
}

/// Writes an RGBA frame as a PNG readable only by the user.
pub fn write_png(path: &Path, f: &Frame) -> std::io::Result<()> {
    crate::persistence::atomic::write_with(path, |file| {
        let mut enc = png::Encoder::new(std::io::BufWriter::new(file), f.width, f.height);
        enc.set_color(png::ColorType::Rgba);
        enc.set_depth(png::BitDepth::Eight);
        enc.set_compression(png::Compression::Fast);
        let mut w = enc.write_header().map_err(std::io::Error::other)?;
        w.write_image_data(&f.rgba).map_err(std::io::Error::other)?;
        w.finish().map_err(std::io::Error::other)
    })
}
