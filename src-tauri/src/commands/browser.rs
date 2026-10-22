use std::sync::Arc;

use serde::Deserialize;
use sonata_core::model::node::{Frame, Node};
use sonata_core::{App, AppError};
use tauri::State;
use uuid::Uuid;

use crate::browser::{Browsers, Rect};

#[tauri::command]
pub fn node_create_browser(
    app: State<'_, Arc<App>>,
    workspace_id: Uuid,
    frame: Frame,
    url: Option<String>,
) -> Result<Node, AppError> {
    app.create_browser(workspace_id, frame, url.as_deref())
}

/// Address bar: normalizes what the user typed and navigates (FR-002).
#[tauri::command]
pub async fn browser_navigate(
    browsers: State<'_, Arc<Browsers>>,
    node_id: Uuid,
    input: String,
) -> Result<String, AppError> {
    let url = sonata_core::browser::normalize_url(&input)?;
    browsers
        .navigate(node_id, &url)
        .map_err(AppError::internal)?;
    Ok(url)
}

#[tauri::command]
pub async fn browser_action(
    browsers: State<'_, Arc<Browsers>>,
    node_id: Uuid,
    action: String,
) -> Result<(), AppError> {
    browsers
        .action(node_id, &action)
        .map_err(AppError::internal)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LayoutItem {
    node_id: Uuid,
    rect: Option<Rect>,
}

/// Live pages and their rects from the frontend (research R7). Async: creating a webview must
/// not block the main thread.
#[tauri::command]
pub async fn browser_layout(
    browsers: State<'_, Arc<Browsers>>,
    items: Vec<LayoutItem>,
) -> Result<(), AppError> {
    browsers.layout(items.into_iter().map(|i| (i.node_id, i.rect)).collect());
    Ok(())
}
