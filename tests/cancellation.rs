// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! Cancelling a task, through a stand-in for Cargo that pauses its build while this process is signalled. This file has a single test, since the signal and the environment are process-wide; it sets the environment before any thread starts.

use bake_benchmark as _;
use base_benchmark::Cancellation;
use std::os::unix::fs::PermissionsExt;
use std::process::Command;
use std::sync::mpsc;
use std::time::{Duration, Instant};

const MANIFEST: &str = "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[[package.metadata.base-benchmark.scenario]]\nname = \"work\"\n";

/// Long enough for any host to handle a signal; reaching it means the signal was never handled, not that handling was slow.
const WATCHDOG: Duration = Duration::from_secs(300);

enum Preparation {
    Paused,
    Finished(Result<(), String>),
}

#[test]
fn a_preparation_signalled_during_its_build_fails_as_cancelled() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("project");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("Cargo.toml"), MANIFEST).unwrap();
    std::fs::write(
        root.join("Cargo.lock"),
        "version = 4\n\n[[package]]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(root.join("src/main.rs"), "fn main() {}\n").unwrap();

    // Cargo's first build reports that it started and waits to be resumed, then builds; Cargo itself is never signalled, as when a supervisor signals only the task.
    let real = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let started = directory.path().join("started");
    let resume = directory.path().join("resume");
    for fifo in [&started, &resume] {
        assert!(Command::new("mkfifo").arg(fifo).status().unwrap().success());
    }
    let paused = directory.path().join("paused");
    let cargo = directory.path().join("cargo");
    std::fs::write(
        &cargo,
        format!(
            "#!/bin/sh\nif [ \"$1\" = build ] && [ ! -e '{paused}' ]; then\n  : > '{paused}'\n  echo started > '{started}'\n  read resumed < '{resume}'\nfi\nexec '{real}' \"$@\"\n",
            paused = paused.display(),
            started = started.display(),
            resume = resume.display(),
            real = std::path::Path::new(&real).display(),
        ),
    )
    .unwrap();
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).unwrap();
    // SAFETY: the only other thread is libtest's main thread, which waits for this test and never touches the environment; no other thread has started yet, and the environment is never written again.
    unsafe { std::env::set_var("CARGO", &cargo) };

    let (sender, receiver) = mpsc::channel();
    std::thread::spawn({
        let sender = sender.clone();
        let root = root.clone();
        move || {
            let result = bake::Registry::discover()
                .unwrap()
                .context(&root)
                .call("benchmark:prepare", &["--output", "target/prepared"])
                .map(|_| ())
                .map_err(|error| error.to_string());
            let _ = sender.send(Preparation::Finished(result));
        }
    });
    // Waiting for the build in its own thread, so that a preparation that fails before the build starts fails the test instead of leaving it waiting.
    std::thread::spawn(move || {
        std::fs::read_to_string(&started).unwrap();
        let _ = sender.send(Preparation::Paused);
    });
    match receiver
        .recv_timeout(WATCHDOG)
        .expect("the build never started")
    {
        Preparation::Paused => {}
        Preparation::Finished(result) => {
            panic!("preparation finished before the build: {result:?}")
        }
    }

    // The task registered its cancellation before building, and the signal thread cancels registrations in order under one lock, so once this later registration is cancelled the task's is too.
    let cancellation = Cancellation::new();
    let signals = cancellation.on_termination_signals();
    assert!(
        Command::new("kill")
            .args(["-TERM", &std::process::id().to_string()])
            .status()
            .unwrap()
            .success()
    );
    let deadline = Instant::now() + WATCHDOG;
    while !cancellation.is_cancelled() {
        assert!(Instant::now() < deadline, "the signal was never handled");
        std::thread::sleep(Duration::from_millis(10));
    }
    drop(signals);
    std::fs::write(&resume, "resumed\n").unwrap();

    let error = match receiver
        .recv_timeout(WATCHDOG)
        .expect("the build never finished")
    {
        Preparation::Finished(result) => result.unwrap_err(),
        Preparation::Paused => unreachable!("the build pauses once"),
    };
    // Cargo finished the build, which does not make the preparation succeed, and no manifest is written.
    assert_eq!(error, "benchmark:prepare: preparation was cancelled");
    assert!(
        root.join("target/prepared/artifacts/current/0/fixture")
            .exists()
    );
    assert!(!root.join("target/prepared/artifacts.json").exists());
}
