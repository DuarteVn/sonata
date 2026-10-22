mod common;

use common::{Sandbox, mode, test_app};
use sonata_core::ErrorCode;
use sonata_core::model::node::{Frame, NoteColor, NoteContent};
use sonata_core::workspace::manager::CreateWorkspace;

const PNG: &[u8] = &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 0];

fn setup(sb: &Sandbox) -> (std::sync::Arc<sonata_core::App>, uuid::Uuid, uuid::Uuid) {
    let (app, _) = test_app(sb);
    let ws = app
        .create_workspace(CreateWorkspace {
            name: "W".into(),
            working_directory: sb.project("p"),
        })
        .unwrap()
        .id;
    app.open_workspace(ws).unwrap();
    let note = app
        .create_note(
            ws,
            Frame {
                x: 0.0,
                y: 0.0,
                width: 300.0,
                height: 200.0,
            },
        )
        .unwrap()
        .id;
    (app, ws, note)
}

#[test]
fn images_are_validated_and_copied() {
    let sb = Sandbox::new();
    let (app, ws, note) = setup(&sb);
    let src = sb.tmp.path().join("shot.png");
    std::fs::write(&src, PNG).unwrap();
    let md = app.note_insert_image(note, &src).unwrap();
    let name = md
        .strip_prefix("![](../files/")
        .and_then(|r| r.strip_suffix(')'))
        .expect(&md);
    assert!(name.ends_with(".png"));
    let copied = sb.root.workspace_dir(ws).join("files").join(name);
    assert_eq!(std::fs::read(&copied).unwrap(), PNG);
    assert_eq!(mode(&copied), 0o600);
    assert_eq!(std::fs::read(&src).unwrap(), PNG, "source changed");
    let second = app.note_insert_image(note, &src).unwrap();
    assert_ne!(second, md, "names must be unique");

    let fake = sb.tmp.path().join("fake.png");
    std::fs::write(&fake, b"not an image").unwrap();
    assert_eq!(
        app.note_insert_image(note, &fake).unwrap_err().code,
        ErrorCode::ImageUnsupported
    );
    let txt = sb.tmp.path().join("a.txt");
    std::fs::write(&txt, PNG).unwrap();
    assert_eq!(
        app.note_insert_image(note, &txt).unwrap_err().code,
        ErrorCode::ImageUnsupported
    );

    let big = sb.tmp.path().join("big.png");
    let f = std::fs::File::create(&big).unwrap();
    f.set_len(21 * 1024 * 1024).unwrap();
    std::fs::write(&big, [PNG, &vec![0u8; 21 * 1024 * 1024][..]].concat()).unwrap();
    assert_eq!(
        app.note_insert_image(note, &big).unwrap_err().code,
        ErrorCode::ImageTooLarge
    );
    app.quit();
}

#[test]
fn style_persists_and_asset_base_is_reported() {
    let sb = Sandbox::new();
    let (app, ws, note) = setup(&sb);
    app.note_set_style(
        note,
        NoteContent {
            color: NoteColor::Blue,
            text_size: 5,
        },
    )
    .unwrap();
    assert!(
        app.note_set_style(
            note,
            NoteContent {
                color: NoteColor::Blue,
                text_size: 9
            }
        )
        .is_err()
    );
    let style = app
        .workspaces()
        .workspace(ws)
        .unwrap()
        .node(note)
        .unwrap()
        .note_style()
        .unwrap()
        .clone();
    assert_eq!(
        style,
        NoteContent {
            color: NoteColor::Blue,
            text_size: 5
        }
    );
    assert_eq!(
        app.note_read(note).unwrap().asset_base,
        sb.root.notes_dir(ws)
    );
    app.quit();
}
