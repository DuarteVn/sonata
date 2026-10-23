//! Last readings and sent warnings, kept in `usage-cache.json` without credentials (FR-014).

use std::collections::BTreeMap;
use std::io;

use serde::{Deserialize, Serialize};

use super::window::WarningEntry;
use super::{Reading, ReadingStatus};
use crate::paths::DataRoot;
use crate::persistence::store::{Loaded, Store};

pub const CACHE_TYPE: &str = "usageCache";
pub const CACHE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct UsageCache {
    #[serde(skip_serializing_if = "BTreeMap::is_empty")]
    pub readings: BTreeMap<String, Reading>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub warnings: Vec<WarningEntry>,
}

/// Readings worth showing after a restart. Failures are not kept: the last good value is.
pub fn is_cacheable(status: ReadingStatus) -> bool {
    matches!(
        status,
        ReadingStatus::Ok | ReadingStatus::NoPlanLimits | ReadingStatus::NotLoggedIn
    )
}

/// Missing file: empty cache. Invalid file: kept aside, empty cache, and a warning.
pub fn load(root: &DataRoot) -> Loaded<UsageCache> {
    Store::new(root.clone()).load_settings(
        &root.usage_cache_file(),
        CACHE_TYPE,
        CACHE_SCHEMA_VERSION,
    )
}

/// Writes the cache atomically, without emails and without non-lasting readings.
pub fn save(root: &DataRoot, cache: &UsageCache) -> io::Result<()> {
    let clean = UsageCache {
        readings: cache
            .readings
            .iter()
            .filter(|(_, r)| is_cacheable(r.status))
            .map(|(id, r)| {
                let mut r = r.clone();
                r.account_email = None;
                (id.clone(), r)
            })
            .collect(),
        warnings: cache.warnings.clone(),
    };
    Store::new(root.clone()).save_settings(
        &root.usage_cache_file(),
        CACHE_TYPE,
        CACHE_SCHEMA_VERSION,
        &clean,
    )
}
