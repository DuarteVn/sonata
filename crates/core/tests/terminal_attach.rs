mod common;

use std::sync::{Arc, Mutex};

use common::{Sandbox, WAIT, wait_until};
use sonata_core::events::NullSink;
use sonata_core::terminal::manager::{OutputSink, SpawnSpec, TerminalManager};
use uuid::Uuid;

#[derive(Clone, Default)]
struct Collect(Arc<Mutex<Vec<Vec<u8>>>>);

impl OutputSink for Collect {
    fn send(&self, bytes: Vec<u8>) -> bool {
        self.0.lock().unwrap().push(bytes);
        true
    }
}

impl Collect {
    fn messages(&self) -> Vec<Vec<u8>> {
        self.0.lock().unwrap().clone()
    }
    fn text(&self) -> String {
        self.messages()
            .iter()
            .map(|m| String::from_utf8_lossy(m).into_owned())
            .collect()
    }
}

fn start(sb: &Sandbox) -> (TerminalManager, Uuid) {
    let m = TerminalManager::new(Arc::new(NullSink));
    let spec = SpawnSpec {
        node_id: Uuid::new_v4(),
        cwd: sb.project("a"),
        launch_command: None,
        rows: 30,
        cols: 100,
        history_max_rows: 10_000,
        shell: Some("/bin/sh".into()),
        restored_history: None,
        session_env: Vec::new(),
        wrap_agents: false,
        shell_helpers: None,
        resume: None,
    };
    let id = spec.node_id;
    m.start(spec).unwrap();
    (m, id)
}

#[test]
fn replay_first_then_live_bytes() {
    let sb = Sandbox::new();
    let (m, id) = start(&sb);
    m.write(id, b"echo before-attach\r").unwrap();
    assert!(wait_until(WAIT, || m
        .screen_text(id)
        .unwrap()
        .lines()
        .any(|l| common::is_output(l, "before-attach"))));

    let sink = Collect::default();
    m.attach(id, Box::new(sink.clone())).unwrap();
    let first = String::from_utf8_lossy(&sink.messages()[0]).into_owned();
    assert!(
        first.starts_with("\x1b[0m\x1b[H\x1b[2J"),
        "replay header missing"
    );
    assert!(first.contains("before-attach"));

    m.write(id, b"echo after-attach\r").unwrap();
    assert!(wait_until(WAIT, || sink
        .text()
        .lines()
        .any(|l| common::is_output(l, "after-attach"))));
    m.stop(id);
}

#[test]
fn output_is_batched_in_chunks_of_at_most_32k() {
    let sb = Sandbox::new();
    let (m, id) = start(&sb);
    let sink = Collect::default();
    m.attach(id, Box::new(sink.clone())).unwrap();
    m.write(id, b"i=0; while [ $i -lt 3000 ]; do echo line-$i-xxxxxxxxxxxxxxxxxxxxxxxxxxxxxx; i=$((i+1)); done; echo DONE-MARK\r")
        .unwrap();
    assert!(wait_until(std::time::Duration::from_secs(15), || sink
        .text()
        .lines()
        .any(|l| common::is_output(l, "DONE-MARK"))));
    let live = &sink.messages()[1..];
    assert!(live.iter().all(|m| m.len() <= 32 * 1024));
    assert!(live.len() < 3000, "not batched: {} messages", live.len());
    m.stop(id);
}

#[test]
fn detach_keeps_process_and_reattach_shows_missed_output() {
    let sb = Sandbox::new();
    let (m, id) = start(&sb);
    let first = Collect::default();
    m.attach(id, Box::new(first.clone())).unwrap();
    m.detach(id);
    m.write(id, b"echo while-detached\r").unwrap();
    assert!(wait_until(WAIT, || m
        .screen_text(id)
        .unwrap()
        .lines()
        .any(|l| common::is_output(l, "while-detached"))));
    assert!(
        !first
            .text()
            .lines()
            .any(|l| common::is_output(l, "while-detached"))
    );

    let second = Collect::default();
    m.attach(id, Box::new(second.clone())).unwrap();
    assert!(String::from_utf8_lossy(&second.messages()[0]).contains("while-detached"));
    m.stop(id);
}
