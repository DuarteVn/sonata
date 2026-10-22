//! Core-side scale check for SC-004: 100 terminals printing continuously.
//! Ignored by default (slow, CPU heavy). Run: cargo test -p sonata-core --test scale -- --ignored --nocapture

mod common;

use std::time::{Duration, Instant};

use common::{Sandbox, test_app, wait_until};
use sonata_core::app::NewTerminal;
use sonata_core::workspace::manager::CreateWorkspace;

fn cpu_seconds() -> f64 {
    // SAFETY: getrusage only writes into the provided struct.
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    let t = |tv: libc::timeval| tv.tv_sec as f64 + tv.tv_usec as f64 / 1e6;
    t(usage.ru_utime) + t(usage.ru_stime)
}

#[test]
#[ignore]
fn hundred_streaming_terminals() {
    let sb = Sandbox::new();
    let (app, _) = test_app(&sb);
    let dir = sb.project("p");
    let mut ids = Vec::new();
    for w in 0..4 {
        let ws = app
            .create_workspace(CreateWorkspace {
                name: format!("W{w}"),
                working_directory: dir.clone(),
            })
            .unwrap()
            .id;
        app.open_workspace(ws).unwrap();
        for i in 0..25 {
            let cmd = format!("i=0; while :; do i=$((i+1)); echo w{w}-t{i}-$i; sleep 0.05; done");
            ids.push(
                app.create_terminal(
                    ws,
                    NewTerminal {
                        launch_command: Some(cmd),
                        ..Default::default()
                    },
                )
                .unwrap()
                .id,
            );
        }
    }
    let started = Instant::now();
    let all_running = wait_until(Duration::from_secs(20), || {
        ids.iter().all(|id| {
            app.terminals()
                .screen_text(*id)
                .is_some_and(|s| s.contains("-1"))
        })
    });
    println!(
        "100 terminals producing output after {:?}: {all_running}",
        started.elapsed()
    );
    assert!(all_running);

    let cpu0 = cpu_seconds();
    let t0 = Instant::now();
    let seq0: Vec<u64> = ids
        .iter()
        .map(|id| app.terminals().output_seq(*id).unwrap())
        .collect();
    std::thread::sleep(Duration::from_secs(10));
    let cpu = (cpu_seconds() - cpu0) / t0.elapsed().as_secs_f64() * 100.0;
    let advanced = ids
        .iter()
        .zip(&seq0)
        .filter(|(id, s0)| app.terminals().output_seq(**id).unwrap() > **s0)
        .count();
    let rss_mb = {
        let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
        unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
        usage.ru_maxrss as f64 / 1024.0 / 1024.0
    };
    println!(
        "core CPU over 10 s: {cpu:.1}% of one core; terminals still updating: {advanced}/100; peak RSS {rss_mb:.0} MB"
    );
    assert_eq!(advanced, 100);
    let t = Instant::now();
    app.save_dirty();
    println!("history save of 100 terminals: {:?}", t.elapsed());
    app.quit();
}
