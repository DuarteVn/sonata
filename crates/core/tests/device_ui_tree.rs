//! Accessibility tree → numbered elements for `sonata device read` (US3, research R4, FR-012).

use sonata_core::device::ui_tree::{format_read, parse_dump};

fn fixture(name: &str) -> String {
    std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/device")
            .join(name),
    )
    .unwrap()
}

#[test]
fn numbers_tappable_scrollable_and_text_elements_in_document_order() {
    let screen = parse_dump(&fixture("settings.xml")).unwrap();
    assert_eq!(screen.app, "com.android.settings");
    assert_eq!(screen.size, (1080, 2400));
    let summary: Vec<(usize, &str, &str)> = screen
        .elements
        .iter()
        .map(|e| (e.number, e.kind.as_str(), e.label.as_str()))
        .collect();
    assert_eq!(
        summary,
        vec![
            // Texts inside a tappable parent join its label and are not numbered.
            (1, "view", "Search settings"),
            (2, "list", "recycler_view"),
            (3, "view", "Network & internet · Mobile, Wi-Fi, hotspot"),
            (4, "view", "Connected devices"),
            // Off-screen and zero-size elements are left out.
            (5, "button", "Navigate up"),
        ]
    );
    let row = &screen.elements[2];
    assert!(row.tappable && !row.editable);
    assert_eq!(row.bounds, (0, 340, 1080, 520));
    assert_eq!(row.center(), (540, 430));
    assert!(screen.elements[1].scrollable);
    assert_eq!(
        screen.elements[0].resource_id.as_deref(),
        Some("search_bar")
    );
}

#[test]
fn inputs_use_hint_or_id_and_flag_password_and_focus() {
    let screen = parse_dump(&fixture("login.xml")).unwrap();
    let text = format_read(&screen);
    assert_eq!(
        text,
        "App: com.example.shop\nScreen: 1080x2400\n\n\
         \x20 [1] text     \"Welcome back\"  #heading  [80,300][1000,400]\n\
         \x20 [2] input    \"Email\"  #email  [80,500][1000,620]  tappable, editable, focused\n\
         \x20 [3] input    \"password\"  #password  [80,660][1000,780]  tappable, editable, password\n\
         \x20 [4] checkbox \"Remember me\"  #remember  [80,820][600,900]  tappable\n\
         \x20 [5] button   \"Sign in\"  #sign_in  [80,960][1000,1080]  tappable"
    );
}

#[test]
fn an_empty_tree_says_so() {
    let screen = parse_dump(&fixture("game.xml")).unwrap();
    assert!(screen.elements.is_empty());
    assert!(format_read(&screen).ends_with(
        "No elements on screen (for example a game or a video). Coordinate taps and screenshots still work."
    ));
}

#[test]
fn output_is_capped_at_100_kb() {
    let mut xml = String::from(
        "<hierarchy rotation=\"0\"><node class=\"android.widget.FrameLayout\" package=\"p\" bounds=\"[0,0][1080,2400]\">",
    );
    for i in 0..3000 {
        xml.push_str(&format!(
            "<node class=\"android.widget.TextView\" text=\"Row number {i} with some longer text to fill the line\" bounds=\"[0,{y}][1080,{y2}]\" />",
            y = i % 2000,
            y2 = i % 2000 + 1
        ));
    }
    xml.push_str("</node></hierarchy>");
    let text = format_read(&parse_dump(&xml).unwrap());
    assert!(text.len() <= 100 * 1024, "{}", text.len());
    assert!(
        text.trim_end()
            .ends_with("more elements; scroll or use coordinates"),
        "{}",
        &text[text.len() - 200..]
    );
}

#[test]
fn broken_xml_is_an_error() {
    assert!(parse_dump("<hierarchy><node").is_err());
    assert!(parse_dump("ERROR: null root node returned by UiTestAutomationBridge.").is_err());
}
