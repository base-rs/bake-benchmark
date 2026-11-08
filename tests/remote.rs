// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! Remote tasks, through stand-ins for `ssh` and for the host's command. This file has a single test, so setting the environment and writing scripts cannot race with another thread.

use bake_benchmark as _;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

const MANIFEST: &str = "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[[package.metadata.base-benchmark.scenario]]\nname = \"work\"\n";
const MAIN: &str = r##"fn main() {
    println!(r#"{{"schema":"base-benchmark/process-record","version":1,"workload":"fixture","parameters":{{}},"samples":[{{"operations":10,"metrics":[{{"name":"wall_time","unit":"nanoseconds","phase":"measured","source":"wall","direction":"lower_is_better","measurement":{{"kind":"total","value":1000}}}}]}}]}}"#);
}
"##;

fn script(path: &Path, body: &str) -> String {
    std::fs::write(path, body).unwrap();
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755)).unwrap();
    path.display().to_string()
}

#[test]
fn remote_tasks_run_on_the_host_and_return_what_local_tasks_return() {
    let directory = tempfile::tempdir().unwrap();
    let root = directory.path().join("project");
    std::fs::create_dir_all(root.join("src")).unwrap();
    std::fs::write(root.join("Cargo.toml"), MANIFEST).unwrap();
    std::fs::write(
        root.join("Cargo.lock"),
        "version = 4\n\n[[package]]\nname = \"fixture\"\nversion = \"0.1.0\"\n",
    )
    .unwrap();
    std::fs::write(root.join("src/main.rs"), MAIN).unwrap();
    let mut context = bake::Registry::discover().unwrap().context(&root);

    // Outputs made locally serve as what the host's command produces.
    let templates = directory.path().join("templates");
    let template = |name: &str| templates.join(name).display().to_string();
    context
        .call("benchmark:prepare", &["--output", &template("prepare")])
        .unwrap();
    context
        .call(
            "benchmark:run",
            &[
                "work",
                "--artifacts",
                &format!("{}/artifacts.json", template("prepare")),
                "--rounds",
                "1",
                "--lock",
                "lock",
                "--output",
                &template("run"),
            ],
        )
        .unwrap();
    context
        .call(
            "benchmark:smoke",
            &["--lock", "lock", "--output", &template("smoke")],
        )
        .unwrap();

    let ssh = script(
        &directory.path().join("ssh"),
        "#!/bin/sh\nshift\nexec \"$@\"\n",
    );
    // SAFETY: this is the test binary's only test, so no other thread reads the environment.
    unsafe { std::env::set_var("BASE_BENCHMARK_SSH", &ssh) };
    let program = script(
        &directory.path().join("program"),
        &format!(
            "#!/bin/sh\ncommand=$1\nshift\nwhile [ $# -gt 0 ]; do case \"$1\" in --output) output=$2; shift;; esac; shift; done\nmkdir -p \"$(dirname \"$output\")\" && cp -R {templates}/\"$command\" \"$output\"\necho \"$command on the host\"\n",
            templates = templates.display()
        ),
    );
    let host = directory.path().join("host").display().to_string();
    let remote = [
        "--host",
        "bench",
        "--remote-directory",
        host.as_str(),
        "--remote-program",
        program.as_str(),
    ];
    let call = |context: &mut bake::Context, task: &str, arguments: &[&str]| {
        let arguments: Vec<&str> = arguments.iter().chain(remote.iter()).copied().collect();
        context.call(task, &arguments)
    };

    let prepared = call(
        &mut context,
        "benchmark:prepare",
        &[
            "--tree",
            "current=.",
            "--scenario",
            "work",
            "--replicas",
            "0",
            "--output",
            "target/remote-prepared",
        ],
    )
    .unwrap();
    assert_eq!(prepared["manifest"]["artifacts"][0]["label"], "current");
    assert!(root.join("target/remote-prepared/artifacts.json").is_file());
    let campaign = call(
        &mut context,
        "benchmark:run",
        &[
            "work",
            "--artifacts",
            "target/remote-prepared/artifacts.json",
            "--mode",
            "screening",
            "--rounds",
            "1",
            "--seed",
            "1",
            "--cpus",
            "0",
            "--timeout-seconds",
            "60",
            "--perf-event",
            "instructions",
            "--lock",
            "lock",
            "--output",
            "target/remote-run",
            "--decision-rule",
            "rule",
            "--idle-host",
            "true",
            "--stack-padding",
            "false",
        ],
    )
    .unwrap();
    assert!(
        campaign["evidence"]
            .as_str()
            .unwrap()
            .starts_with("screening"),
        "the downloaded campaign is the template, a screening run: {campaign}"
    );
    let campaigns = call(
        &mut context,
        "benchmark:smoke",
        &["--lock", "lock", "--output", "target/remote-smoke"],
    )
    .unwrap();
    assert_eq!(campaigns.as_array().unwrap().len(), 1);

    // Errors are reported, here a tree that is not LABEL=PATH, before the host is contacted.
    let error = call(&mut context, "benchmark:prepare", &["--tree", "current"]).unwrap_err();
    assert!(
        error.to_string().contains("LABEL=PATH") || error.to_string().contains("tree"),
        "{error}"
    );
}
