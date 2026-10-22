//! Role library: named, reusable instructions shared by all workspaces (`roles.json`).

use std::io;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::{AppError, AppResult, ErrorCode};
use crate::model::node::ROLE_TEXT_MAX_CHARS;
use crate::persistence::migrate::Migrations;
use crate::persistence::{atomic, envelope};

pub const ROLES_TYPE: &str = "roles";
pub const ROLES_SCHEMA_VERSION: u32 = 1;
pub const ROLE_NAME_MAX_CHARS: usize = 60;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    pub id: Uuid,
    pub name: String,
    pub text: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct RolesFile {
    #[serde(default)]
    roles: Vec<Role>,
}

pub struct RoleLibrary {
    path: PathBuf,
    roles: Vec<Role>,
}

impl RoleLibrary {
    /// Loads `roles.json`; a missing file is an empty library. An invalid file is reported and
    /// left untouched until the next save.
    pub fn load(path: PathBuf) -> (Self, Option<String>) {
        let (roles, warning) = match std::fs::read(&path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => (Vec::new(), None),
            Err(e) => (
                Vec::new(),
                Some(format!("Could not read the role library: {e}")),
            ),
            Ok(bytes) => match envelope::decode::<RolesFile>(
                &bytes,
                ROLES_TYPE,
                &Migrations::new(ROLES_SCHEMA_VERSION),
            ) {
                Ok(f) => (f.roles, None),
                Err(e) => (
                    Vec::new(),
                    Some(format!("The role library is invalid ({e}).")),
                ),
            },
        };
        (Self { path, roles }, warning)
    }

    pub fn list(&self) -> Vec<Role> {
        self.roles.clone()
    }

    pub fn save(&mut self, name: &str, text: &str) -> AppResult<Role> {
        let name = name.trim();
        if name.is_empty() {
            return Err(AppError::new(ErrorCode::NameEmpty, "Enter a role name."));
        }
        if name.chars().count() > ROLE_NAME_MAX_CHARS {
            return Err(AppError::new(
                ErrorCode::NameTooLong,
                "Role names can have at most 60 characters.",
            ));
        }
        if text.trim().is_empty() {
            return Err(AppError::new(
                ErrorCode::NameEmpty,
                "Write the role text first.",
            ));
        }
        if text.chars().count() > ROLE_TEXT_MAX_CHARS {
            return Err(AppError::new(
                ErrorCode::NameTooLong,
                "Roles can have at most 20,000 characters.",
            ));
        }
        let lower = name.to_lowercase();
        if self.roles.iter().any(|r| r.name.to_lowercase() == lower) {
            return Err(AppError::new(
                ErrorCode::RoleNameTaken,
                format!("A role named \"{name}\" already exists."),
            ));
        }
        let role = Role {
            id: Uuid::new_v4(),
            name: name.to_string(),
            text: text.to_string(),
            created_at: Utc::now(),
        };
        self.roles.push(role.clone());
        if let Err(e) = self.persist() {
            self.roles.pop();
            return Err(e.into());
        }
        Ok(role)
    }

    pub fn delete(&mut self, id: Uuid) -> AppResult<()> {
        let before = self.roles.clone();
        self.roles.retain(|r| r.id != id);
        if let Err(e) = self.persist() {
            self.roles = before;
            return Err(e.into());
        }
        Ok(())
    }

    fn persist(&self) -> io::Result<()> {
        let file = RolesFile {
            roles: self.roles.clone(),
        };
        let bytes =
            envelope::encode(ROLES_TYPE, ROLES_SCHEMA_VERSION, &file).map_err(io::Error::other)?;
        atomic::write(&self.path, &bytes)
    }
}
