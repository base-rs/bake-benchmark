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
                "--scenario",
                "work",
                "--overlay",
                "false",
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
                &allowed_cpu(),
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

fn error(context: &mut bake::Context, task: &str, arguments: &[&str]) -> String {
    context.call(task, arguments).unwrap_err().to_string()
}

#[test]
fn task_errors_come_from_the_shared_implementation() {
    let directory = tempfile::tempdir().unwrap();
    let mut context = context(directory.path());
    assert!(error(&mut context, "benchmark:list", &[]).contains("cargo metadata failed"));
    assert!(error(&mut context, "benchmark:prepare", &[]).contains("cargo metadata failed"));
    assert!(
        error(&mut context, "benchmark:smoke", &["--lock", "lock"])
            .contains("cargo metadata failed")
    );
    assert!(
        error(
            &mut context,
            "benchmark:run",
            &["work", "--artifacts", "a.json", "--cpus", "x"]
        )
        .contains("invalid CPU list")
    );
    assert!(
        error(
            &mut context,
            "benchmark:run",
            &["work", "--artifacts", "a.json", "--lock", "lock"]
        )
        .contains("cargo metadata failed")
    );
    assert!(error(&mut context, "benchmark:summarize", &["missing"]).contains("metadata.json"));
}

#[test]
fn standalone_manifests_absolute_paths_and_recorded_prerequisites() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("project");
    // A suite in its own workspace, as for benchmarks excluded from a project's main workspace.
    fixture(&root.join("suite"));
    let mut context = context(&root);
    let manifest = ["--manifest-path", "suite/Cargo.toml"];
    let suites = context.call("benchmark:list", &manifest).unwrap();
    assert_eq!(suites[0]["scenarios"][0]["baseline"], "reference");

    let output = directory.path().join("prepared");
    let output = output.to_str().unwrap();
    context
        .call(
            "benchmark:prepare",
            &["--manifest-path", "suite/Cargo.toml", "--output", output],
        )
        .unwrap();
    let artifacts = format!("{output}/artifacts.json");
    let recorded = error(
        &mut context,
        "benchmark:run",
        &[
            "work",
            "--artifacts",
            &artifacts,
            "--manifest-path",
            "suite/Cargo.toml",
            "--mode",
            "recorded",
            "--lock",
            "lock",
        ],
    );
    assert!(
        recorded.contains("recorded campaigns need explicit CPU placement"),
        "{recorded}"
    );

    // An unavailable perf event fails the sample; the request still reaches the plan.
    let campaign = directory.path().join("perf");
    let arguments = [
        "work",
        "--artifacts",
        &artifacts,
        "--manifest-path",
        "suite/Cargo.toml",
        "--rounds",
        "1",
        "--perf-event",
        "not-an-event:u",
        "--stack-padding",
        "false",
        "--lock",
        "lock",
        "--output",
        campaign.to_str().unwrap(),
    ];
    assert!(context.call("benchmark:run", &arguments).is_err());
    let metadata: serde_json::Value =
        serde_json::from_slice(&std::fs::read(campaign.join("metadata.json")).unwrap()).unwrap();
    assert_eq!(metadata["plan"]["perf_events"][0], "not-an-event:u");
    assert_eq!(metadata["plan"]["stack_padding"], false);
}

fn cli(arguments: &[&str]) -> (bool, String) {
    let mut errors = Vec::new();
    let arguments = arguments
        .iter()
        .map(|argument| (*argument).to_owned())
        .collect();
    let code = base_benchmark::cli::main(arguments, &mut Vec::new(), &mut errors);
    (
        code == std::process::ExitCode::SUCCESS,
        String::from_utf8(errors).unwrap(),
    )
}

/// The command and the task produce the same summary and the same errors from equivalent plans.
#[test]
fn the_command_and_the_tasks_agree_under_equivalent_plans() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("project");
    fixture(&root);
    let mut context = context(&root);
    let path = |name: &str| directory.path().join(name).display().to_string();
    let manifest = root.join("Cargo.toml").display().to_string();
    context
        .call("benchmark:prepare", &["--output", &path("prepared")])
        .unwrap();
    let artifacts = path("prepared/artifacts.json");
    let lock = path("lock");
    let cpu = allowed_cpu();
    let plan = [
        "--artifacts",
        &artifacts,
        "--rounds",
        "2",
        "--seed",
        "11",
        "--cpus",
        &cpu,
        "--timeout-seconds",
        "60",
        "--decision-rule",
        "descriptive",
        "--stack-padding",
        "true",
        "--package",
        "fixture",
        "--lock",
        &lock,
    ];

    let mut task = vec!["work", "--output"];
    let task_output = path("task");
    task.push(&task_output);
    task.extend(plan);
    context.call("benchmark:run", &task).unwrap();
    let mut command = vec!["run", "work", "--manifest-path", &manifest, "--output"];
    let command_output = path("command");
    command.push(&command_output);
    command.extend(plan);
    let (succeeded, errors) = cli(&command);
    assert!(succeeded, "{errors}");
    let summary = |output: &str| -> serde_json::Value {
        serde_json::from_slice(&std::fs::read(Path::new(output).join("summary.json")).unwrap())
            .unwrap()
    };
    assert_eq!(summary(&task_output), summary(&command_output));
    // Every plan setting, not only those visible in the summary, must match; only the invocation names its own command.
    let plan = |output: &str| -> serde_json::Value {
        let mut metadata: serde_json::Value = serde_json::from_slice(
            &std::fs::read(Path::new(output).join("metadata.json")).unwrap(),
        )
        .unwrap();
        metadata["plan"]
            .as_object_mut()
            .unwrap()
            .remove("invocation");
        metadata["plan"].take()
    };
    assert_eq!(plan(&task_output), plan(&command_output));
    assert_eq!(plan(&task_output)["timeout_ms"], 60_000);

    let invalid = [
        "work",
        "--artifacts",
        &artifacts,
        "--mode",
        "recorded",
        "--lock",
        &lock,
    ];
    let task_error = error(&mut context, "benchmark:run", &invalid);
    let mut command = vec!["run", "--manifest-path", &manifest];
    command.extend(invalid);
    let (succeeded, command_error) = cli(&command);
    assert!(!succeeded);
    let message = task_error.trim_start_matches("benchmark:run: ");
    assert!(message.contains("recorded campaigns need"), "{message}");
    assert_eq!(
        command_error.trim_end(),
        format!("base-benchmark: {message}")
    );
}

/// A CPU this test process may run on, since hosts and containers do not always allow CPU 0.
fn allowed_cpu() -> String {
    let status = std::fs::read_to_string("/proc/self/status").unwrap();
    let list = status
        .lines()
        .find_map(|line| line.strip_prefix("Cpus_allowed_list:"))
        .unwrap();
    list.trim().split([',', '-']).next().unwrap().to_owned()
}
