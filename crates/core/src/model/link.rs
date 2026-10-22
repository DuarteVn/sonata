use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// An undirected connection between two nodes of one workspace. Stored with `a < b`, so each
/// pair has one canonical form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Link {
    pub id: Uuid,
    pub a: Uuid,
    pub b: Uuid,
    pub created_at: DateTime<Utc>,
}

impl Link {
    pub fn new(x: Uuid, y: Uuid) -> Self {
        let (a, b) = if x < y { (x, y) } else { (y, x) };
        Self {
            id: Uuid::new_v4(),
            a,
            b,
            created_at: Utc::now(),
        }
    }

    pub fn joins(&self, x: Uuid, y: Uuid) -> bool {
        (self.a == x && self.b == y) || (self.a == y && self.b == x)
    }

    pub fn touches(&self, node: Uuid) -> bool {
        self.a == node || self.b == node
    }

    /// The node at the other end, if `node` is one end.
    pub fn other(&self, node: Uuid) -> Option<Uuid> {
        if self.a == node {
            Some(self.b)
        } else if self.b == node {
            Some(self.a)
        } else {
            None
        }
    }
}
