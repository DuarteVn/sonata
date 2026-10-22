// Declares the app's commands so each one needs an explicit permission in
// capabilities/default.json (contracts/ipc.md lists them).
const COMMANDS: &[&str] = &[
    "app_bootstrap",
    "app_quit_check",
    "app_quit",
    "app_reveal_path",
    "workspace_create",
    "workspace_open",
    "workspace_rename",
    "workspace_set_pinned",
    "workspace_reorder",
    "workspace_delete",
    "node_create_terminal",
    "node_update_frame",
    "node_bring_to_front",
    "node_delete",
    "workspace_set_camera",
    "node_create_note",
    "note_read",
    "note_write",
    "terminal_attach",
    "terminal_detach",
    "terminal_write",
    "terminal_resize",
    "terminal_rename",
    "terminal_set_launch_command",
    "terminal_restart",
    "terminal_set_save_history",
    "terminal_clear_history",
    "terminal_preview_subscribe",
];

fn main() {
    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(COMMANDS)),
    )
    .expect("failed to run tauri-build");
}
