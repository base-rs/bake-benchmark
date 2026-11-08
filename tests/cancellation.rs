// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! Cancelling a task, through a stand-in for Cargo that signals the task's process. This file has a single test, since the signal and the environment are process-wide; it sets the environment before any task starts a thread.

use bake_benchmark as _;
use std::os::unix::fs::PermissionsExt;

const MANIFEST: &str = "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[[package.metadata.base-benchmark.scenario]]\nname = \"work\"\n";

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

    // Cargo's first build sends this process SIGTERM, as a supervisor that signals only the task would, and waits until the signal has been delivered, so that it is handled before the build ends; Cargo itself is not signalled and finishes the build.
    let real = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
    let signalled = directory.path().join("signalled");
    let cargo = directory.path().join("cargo");
    std::fs::write(
        &cargo,
        format!(
            "#!/bin/sh\nif [ \"$1\" = build ] && [ ! -e '{signalled}' ]; then\n  : > '{signalled}'\n  kill -TERM $PPID\n  while [ $(( 0x$(sed -n 's/^ShdPnd:\\t//p' /proc/$PPID/status) & 0x4000 )) -ne 0 ]; do sleep 0.01; done\nfi\nexec '{real}' \"$@\"\n",
            signalled = signalled.display(),
            real = std::path::Path::new(&real).display(),
        ),
    )
    .unwrap();
    std::fs::set_permissions(&cargo, std::fs::Permissions::from_mode(0o755)).unwrap();
    // SAFETY: the only other thread is libtest's main thread, which waits for this test and never touches the environment; no task has started a thread yet, and the environment is never written again.
    unsafe { std::env::set_var("CARGO", &cargo) };

    let mut context = bake::Registry::discover().unwrap().context(&root);
    let error = context
        .call("benchmark:prepare", &["--output", "target/prepared"])
        .unwrap_err()
        .to_string();
    assert!(signalled.exists(), "Cargo built nothing: {error}");
    // The build Cargo finished does not make the preparation succeed, and no manifest is written.
    assert!(
        error.starts_with("benchmark:prepare: preparation was cancelled"),
        "{error}"
    );
    assert!(!root.join("target/prepared/artifacts.json").exists());
}
