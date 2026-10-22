use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::link::Link;
use super::node::Node;

pub const WORKSPACE_TYPE: &str = "workspace";
/// Version 2 (feature 002) added `links`.
pub const WORKSPACE_SCHEMA_VERSION: u32 = 2;

pub const ZOOM_MIN: f64 = 0.1;
pub const ZOOM_MAX: f64 = 4.0;
pub const NAME_MAX_CHARS: usize = 80;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workspace {
    pub id: Uuid,
    pub name: String,
    pub icon: String,
    pub color: String,
    pub working_directory: PathBuf,
    #[serde(default)]
    pub is_pinned: bool,
    pub created_at: DateTime<Utc>,
    pub last_opened_at: DateTime<Utc>,
    #[serde(default)]
    pub camera: Camera,
    #[serde(default)]
    pub nodes: Vec<Node>,
    #[serde(default)]
    pub links: Vec<Link>,
}

impl Workspace {
    pub fn node(&self, id: Uuid) -> Option<&Node> {
        self.nodes.iter().find(|n| n.id == id)
    }

    pub fn node_mut(&mut self, id: Uuid) -> Option<&mut Node> {
        self.nodes.iter_mut().find(|n| n.id == id)
    }

    pub fn max_z(&self) -> i64 {
        self.nodes.iter().map(|n| n.z).max().unwrap_or(0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Camera {
    pub x: f64,
    pub y: f64,
    pub zoom: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            zoom: 1.0,
        }
    }
}

impl Camera {
    /// Zoom clamped to [0.1, 4.0]; non-finite values fall back to defaults.
    pub fn clamped(self) -> Self {
        let finite = |v: f64, d: f64| if v.is_finite() { v } else { d };
        Self {
            x: finite(self.x, 0.0),
            y: finite(self.y, 0.0),
            zoom: finite(self.zoom, 1.0).clamp(ZOOM_MIN, ZOOM_MAX),
        }
    }
}
