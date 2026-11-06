// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

use bake_benchmark as _;
use std::path::Path;

/// A workload that prints a valid record; `fast` lowers its wall time.
const MAIN: &str = r##"fn main() {
    let fast = std::env::args().any(|argument| argument == "fast");
    let wall = if fast { 900 } else { 1000 };
    println!(r#"{{"schema":"base-benchmark/process-record","version":1,"workload":"fixture","parameters":{{}},"samples":[{{"operations":10,"metrics":[{{"name":"wall_time","unit":"nanoseconds","phase":"measured","source":"wall","direction":"lower_is_better","measurement":{{"kind":"total","value":{wall}}}}}]}}]}}"#);
}
"##;

const MANIFEST: &str = r#"[package]
name = "fixture"
version = "0.1.0"
edition = "2021"
publish = false

[[package.metadata.base-benchmark.scenario]]
name = "work"
variants = [{ name = "reference" }, { name = "fast", arguments = ["fast"] }]
"#;

fn fixture(path: &Path) {
    std::fs::create_dir_all(path.join("src")).unwrap();
    std::fs::write(path.join("Cargo.toml"), MANIFEST).unwrap();
    std::fs::write(
        path.join("Cargo.lock"),
        "version = 4\n\n[[package]]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(path.join("src/main.rs"), MAIN).unwrap();
}

fn context(root: &Path) -> bake::Context {
    bake::Registry::discover().unwrap().context(root)
}

#[test]
fn tasks_delegate_to_the_shared_workspace_operations() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("project");
    fixture(&root);
    let mut context = context(&root);

    let suites = context.call("benchmark:list", &[]).unwrap();
    assert_eq!(suites[0]["package"], "fixture");

    let prepared = context
        .call(
            "benchmark:prepare",
            &[
                "--package",
                "fixture",
                "--tree",
                "base=.",
                "--tree",
                "same=.",
                "--output",
                "target/prepared",
            ],
        )
        .unwrap();
    let labels: Vec<_> = prepared["manifest"]["artifacts"]
        .as_array()
        .unwrap()
        .iter()
        .map(|artifact| artifact["label"].as_str().unwrap().to_owned())
        .collect();
    assert_eq!(labels, ["base", "same", "base-replica"]);

    let campaign = context
        .call(
            "benchmark:run",
            &[
                "work",
                "--artifacts",
                "target/prepared/artifacts.json",
                "--mode",
                "screening",
                "--rounds",
                "2",
                "--seed",
                "4",
                "--cpus",
                "0",
                "--timeout-seconds",
                "60",
                "--lock",
                "lock",
                "--output",
                "target/campaign",
                "--decision-rule",
                "descriptive",
                "--idle-host",
                "false",
            ],
        )
        .unwrap();
    let summary_path = campaign["summary_path"].as_str().unwrap();
    let summary: serde_json::Value =
        serde_json::from_slice(&std::fs::read(summary_path).unwrap()).unwrap();
    assert_eq!(summary["rounds"], 2);
    assert!(
        campaign["evidence"]
            .as_str()
            .unwrap()
            .starts_with("screening")
    );
    assert!(campaign["warnings"].as_array().is_some());
    assert!(campaign.get("report").is_none(), "results stay compact");

    let regenerated = context
        .call("benchmark:summarize", &["target/campaign"])
        .unwrap();
    assert_eq!(regenerated, campaign);

    let smoke = context
        .call(
            "benchmark:smoke",
            &["--lock", "lock", "--output", "target/smoke"],
        )
        .unwrap();
    assert!(smoke[0]["evidence"].as_str().unwrap().starts_with("smoke"));
    let default_mode = context
        .call(
            "benchmark:run",
            &[
                "work",
                "--artifacts",
                "target/prepared/artifacts.json",
                "--rounds",
                "1",
                "--lock",
                "lock",
            ],
        )
        .unwrap();
    assert!(
        default_mode["evidence"]
            .as_str()
            .unwrap()
            .starts_with("screening")
    );
}

#[test]
fn task_errors_come_from_the_shared_implementation() {
    let directory = tempfile::tempdir().unwrap();
    let mut context = context(directory.path());
    assert!(
        context
            .call("benchmark:list", &[])
            .unwrap_err()
            .to_string()
            .contains("cargo metadata failed")
    );
    assert!(context.call("benchmark:prepare", &[]).is_err());
    assert!(
        context
            .call("benchmark:smoke", &["--lock", "lock"])
            .is_err()
    );
    assert!(
        context
            .call(
                "benchmark:run",
                &["work", "--artifacts", "a.json", "--cpus", "x"]
            )
            .unwrap_err()
            .to_string()
            .contains("invalid CPU list")
    );
    assert!(
        context
            .call(
                "benchmark:run",
                &["work", "--artifacts", "a.json", "--lock", "lock"]
            )
            .is_err()
    );
    assert!(
        context
            .call("benchmark:summarize", &["missing"])
            .unwrap_err()
            .to_string()
            .contains("metadata.json")
    );
}
