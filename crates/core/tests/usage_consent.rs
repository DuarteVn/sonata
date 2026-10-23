use sonata_core::usage::consent::Consent;

fn argv(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

#[test]
fn granted_only_for_the_exact_command() {
    let c = Consent::granted(argv(&["/bin/tool", "--json"]));
    assert!(c.is_granted_for(&argv(&["/bin/tool", "--json"])));
    assert!(!c.is_granted_for(&argv(&["/bin/tool", "--json", "--more"])));
    assert!(!c.is_granted_for(&argv(&["/bin/tool"])));
    assert!(!Consent::Unasked.is_granted_for(&argv(&["/bin/tool"])));
    assert!(!Consent::Denied.is_granted_for(&argv(&["/bin/tool"])));
}

#[test]
fn a_changed_command_reads_as_unasked() {
    let c = Consent::granted(argv(&["/bin/tool"]));
    assert_eq!(c.state_for(&argv(&["/bin/tool"])), "granted");
    assert_eq!(c.state_for(&argv(&["/bin/other"])), "unasked");
    assert_eq!(Consent::Denied.state_for(&argv(&["/bin/tool"])), "denied");
}

#[test]
fn serializes_with_a_state_tag() {
    let v = serde_json::to_value(Consent::granted(argv(&["a"]))).unwrap();
    assert_eq!(
        v,
        serde_json::json!({"state":"granted","approvedCommand":["a"]})
    );
    let v = serde_json::to_value(Consent::Denied).unwrap();
    assert_eq!(v, serde_json::json!({"state":"denied"}));
}
