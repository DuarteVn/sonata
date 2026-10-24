mod common;

use common::fake_bin::fake_bin;
use sonata_core::usage::ProviderKind;
use sonata_core::usage::custom::provider_infos;

fn write(dir: &std::path::Path, name: &str, payload: &str) {
    std::fs::create_dir_all(dir).unwrap();
    std::fs::write(
        dir.join(name),
        format!(r#"{{"type":"usageProvider","schemaVersion":1,"payload":{payload}}}"#),
    )
    .unwrap();
}

fn errors(dir: &std::path::Path) -> Vec<(String, String)> {
    provider_infos(dir)
        .into_iter()
        .filter_map(|p| p.definition_error.map(|e| (p.name, e)))
        .collect()
}

#[test]
fn a_valid_definition_becomes_a_custom_provider() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("usage-providers");
    let bin = fake_bin(tmp.path(), "my-usage", "exit 0");
    write(
        &dir,
        "mine.json",
        &format!(
            r##"{{"id":"my-agent","name":"My Agent","command":["{}","--json"],"color":"#3B82F6"}}"##,
            bin.display()
        ),
    );
    std::fs::write(dir.join("README.md"), "guide").unwrap();
    std::fs::write(dir.join("notes.txt"), "ignored").unwrap();
    let list = provider_infos(&dir);
    assert_eq!(list.len(), 1);
    let p = &list[0];
    assert_eq!(p.id, "my-agent");
    assert_eq!(p.name, "My Agent");
    assert_eq!(p.kind, ProviderKind::Custom);
    assert_eq!(p.default_color, "#3b82f6");
    assert_eq!(p.command, [bin.display().to_string(), "--json".into()]);
    assert_eq!(p.consent_key, p.command);
    assert_eq!(p.command_path.as_deref(), Some(bin.as_path()));
    assert!(p.definition_error.is_none());
}

#[test]
fn invalid_files_get_an_error_row_and_do_not_affect_others() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("usage-providers");
    let bin = fake_bin(tmp.path(), "tool", "exit 0");
    let b = bin.display();
    let long = "x".repeat(1025);
    let cases = [
        ("a-type.json", None, "{}"),
        (
            "b-id.json",
            Some(format!(r#"{{"id":"Bad_Id","name":"N","command":["{b}"]}}"#)),
            "",
        ),
        (
            "c-builtin.json",
            Some(format!(r#"{{"id":"claude","name":"N","command":["{b}"]}}"#)),
            "",
        ),
        (
            "d-name.json",
            Some(format!(r#"{{"id":"d","name":"","command":["{b}"]}}"#)),
            "",
        ),
        (
            "e-relative.json",
            Some(r#"{"id":"e","name":"N","command":["tool"]}"#.to_string()),
            "",
        ),
        (
            "f-missing.json",
            Some(r#"{"id":"f","name":"N","command":["/nonexistent/tool"]}"#.to_string()),
            "",
        ),
        (
            "g-color.json",
            Some(format!(
                r#"{{"id":"g","name":"N","command":["{b}"],"color":"blue"}}"#
            )),
            "",
        ),
        (
            "h-unknown.json",
            Some(format!(
                r#"{{"id":"h","name":"N","command":["{b}"],"shell":true}}"#
            )),
            "",
        ),
        (
            "i-long.json",
            Some(format!(
                r#"{{"id":"i","name":"N","command":["{b}","{long}"]}}"#
            )),
            "",
        ),
        (
            "j-empty.json",
            Some(r#"{"id":"j","name":"N","command":[]}"#.to_string()),
            "",
        ),
    ];
    for (file, payload, raw) in &cases {
        match payload {
            Some(p) => write(&dir, file, p),
            None => std::fs::write(dir.join(file), raw).unwrap_or_else(|_| {
                std::fs::create_dir_all(&dir).unwrap();
                std::fs::write(dir.join(file), raw).unwrap()
            }),
        }
    }
    write(
        &dir,
        "z-good.json",
        &format!(r#"{{"id":"good","name":"Good","command":["{b}"]}}"#),
    );
    let errs = errors(&dir);
    assert_eq!(errs.len(), cases.len(), "{errs:#?}");
    let good: Vec<_> = provider_infos(&dir)
        .into_iter()
        .filter(|p| p.definition_error.is_none())
        .map(|p| p.id)
        .collect();
    assert_eq!(good, ["good"]);
    let text = |file: &str| errs.iter().find(|(n, _)| n == file).unwrap().1.clone();
    assert!(text("b-id.json").contains("id"), "{}", text("b-id.json"));
    assert!(text("c-builtin.json").contains("built-in"));
    assert!(text("e-relative.json").contains("absolute"));
    assert!(text("f-missing.json").contains("not an executable"));
    assert!(text("g-color.json").contains("color"));
    assert!(text("h-unknown.json").contains("shell"));
}

#[test]
fn duplicate_ids_reject_both_files() {
    let tmp = tempfile::tempdir().unwrap();
    let dir = tmp.path().join("usage-providers");
    let bin = fake_bin(tmp.path(), "tool", "exit 0");
    for f in ["one.json", "two.json"] {
        write(
            &dir,
            f,
            &format!(
                r#"{{"id":"same","name":"N","command":["{}"]}}"#,
                bin.display()
            ),
        );
    }
    let errs = errors(&dir);
    assert_eq!(errs.len(), 2);
    assert!(errs[0].1.contains("two.json"), "{errs:?}");
}

#[test]
fn a_missing_folder_has_no_providers() {
    let tmp = tempfile::tempdir().unwrap();
    assert!(provider_infos(&tmp.path().join("none")).is_empty());
}
