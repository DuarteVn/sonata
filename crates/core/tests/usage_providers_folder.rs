mod common;

use common::Sandbox;

#[test]
fn folder_is_created_private_with_a_guide_that_is_never_overwritten() {
    let sb = Sandbox::new();
    let (app, _) = common::test_app(&sb);
    let dir = app.usage_providers_folder().unwrap();
    assert_eq!(dir, sb.root.usage_providers_dir());
    assert_eq!(common::mode(&dir), 0o700);
    let guide = std::fs::read_to_string(dir.join("README.md")).unwrap();
    assert!(guide.contains("usageProvider"));
    assert!(guide.contains("usedPercent"));
    std::fs::write(dir.join("README.md"), "my notes").unwrap();
    app.usage_providers_folder().unwrap();
    assert_eq!(
        std::fs::read_to_string(dir.join("README.md")).unwrap(),
        "my notes"
    );
}
