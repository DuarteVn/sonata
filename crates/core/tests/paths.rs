mod common;

use std::path::{Path, PathBuf};

use sonata_core::paths::{self, DataRoot, PathError};
use uuid::Uuid;

#[test]
fn sonata_home_overrides_default() {
    let home = Path::new("/Users/someone");
    assert_eq!(
        paths::resolve_root(None, home),
        PathBuf::from("/Users/someone/.sonata")
    );
    assert_eq!(
        paths::resolve_root(Some("/tmp/x/sonata".into()), home),
        PathBuf::from("/tmp/x/sonata")
    );
    assert_eq!(
        paths::resolve_root(Some("".into()), home),
        PathBuf::from("/Users/someone/.sonata")
    );
}

#[test]
fn created_folders_are_private() {
    let tmp = tempfile::tempdir().unwrap();
    let root = DataRoot::at(tmp.path().join("sonata")).unwrap();
    common::assert_private_dir(root.path());
    let ws = root.create_workspace_dir(Uuid::new_v4()).unwrap();
    common::assert_private_dir(&ws);
    common::assert_private_dir(&root.workspaces_dir());
}

#[test]
fn workspace_folder_names_must_be_uuids() {
    let id = Uuid::new_v4();
    assert_eq!(paths::parse_workspace_folder(&id.to_string()), Some(id));
    assert_eq!(paths::parse_workspace_folder("notes"), None);
    assert_eq!(paths::parse_workspace_folder(".trash-x"), None);
}

#[test]
fn parent_components_are_rejected() {
    assert_eq!(paths::reject_parent_components(Path::new("/a/b")), Ok(()));
    assert_eq!(
        paths::reject_parent_components(Path::new("/a/../b")),
        Err(PathError::ParentComponent)
    );
}

#[test]
fn deletion_guard_accepts_only_workspace_folders() {
    let tmp = tempfile::tempdir().unwrap();
    let root = DataRoot::at(tmp.path().join("sonata")).unwrap();
    let id = Uuid::new_v4();
    let good = root.workspace_dir(id);
    assert_eq!(root.check_deletable_workspace_dir(&good), Ok(()));

    let outside = tmp.path().join("projects").join(id.to_string());
    assert!(root.check_deletable_workspace_dir(&outside).is_err());
    let nested = good.join("notes");
    assert!(root.check_deletable_workspace_dir(&nested).is_err());
    let not_uuid = root.workspaces_dir().join("something");
    assert!(root.check_deletable_workspace_dir(&not_uuid).is_err());
    let sneaky = root.workspaces_dir().join("..").join(id.to_string());
    assert!(root.check_deletable_workspace_dir(&sneaky).is_err());
    assert!(root.check_deletable_workspace_dir(root.path()).is_err());
}
