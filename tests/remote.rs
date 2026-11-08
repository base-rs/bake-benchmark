// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! Remote tasks, through stand-ins for `ssh` and for the host's command. This file has a single test, which sets the environment before any task starts a thread, and writes each script before running it.

use bake_benchmark as _;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;

/// Two scenarios, declared out of alphabetical order, so that results in declaration order differ from results in path order.
const MANIFEST: &str = "[package]\nname = \"fixture\"\nversion = \"0.1.0\"\nedition = \"2021\"\npublish = false\n\n[[package.metadata.base-benchmark.scenario]]\nname = \"work\"\n\n[[package.metadata.base-benchmark.scenario]]\nname = \"alpha\"\n";
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
fn remote_tasks_forward_their_options_and_return_what_local_tasks_return() {
    let directory = tempfile::tempdir().unwrap();
    let ssh = script(
        &directory.path().join("ssh"),
        "#!/bin/sh\nshift\nexec \"$@\"\n",
    );
    // SAFETY: no other thread exists yet: libtest runs this test on its own thread, and nothing else has started one. The environment is never written again.
    unsafe { std::env::set_var("BASE_BENCHMARK_SSH", &ssh) };

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

    // The host's command records its arguments, one per line, then copies the template as its output.
    let recorded = directory.path().join("recorded");
    std::fs::create_dir_all(&recorded).unwrap();
    let program = script(
        &directory.path().join("program"),
        &format!(
            "#!/bin/sh\nprintf '%s\\n' \"$@\" > {recorded}/\"$1\"\ncommand=$1\nshift\nwhile [ $# -gt 0 ]; do case \"$1\" in --output) output=$2; shift;; esac; shift; done\nmkdir -p \"$(dirname \"$output\")\" && cp -R {templates}/\"$command\" \"$output\"\necho \"$command on the host\"\n",
            recorded = recorded.display(),
            templates = templates.display()
        ),
    );
    let host = directory.path().join("host");
    let host_text = host.display().to_string();
    let remote = [
        "--host",
        "bench",
        "--remote-directory",
        host_text.as_str(),
        "--remote-program",
        program.as_str(),
    ];
    let call = |context: &mut bake::Context, task: &str, arguments: &[&str]| {
        let arguments: Vec<&str> = arguments.iter().chain(remote.iter()).copied().collect();
        context.call(task, &arguments)
    };
    let mirror = |path: &str| format!("{}/mirror{}/{path}", host.display(), root.display());
    let forwarded = |command: &str| {
        std::fs::read_to_string(recorded.join(command))
            .unwrap()
            .lines()
            .map(str::to_owned)
            .collect::<Vec<_>>()
    };

    // Relative paths are resolved against the project root, then mirrored on the host.
    let prepared = call(
        &mut context,
        "benchmark:prepare",
        &[
            "--package",
            "fixture",
            "--tree",
            "current=.",
            "--tree",
            "other=.",
            "--scenario",
            "work",
            "--scenario",
            "alpha",
            "--replicas",
            "0",
            "--overlay",
            "false",
            "--output",
            "target/remote-prepared",
        ],
    )
    .unwrap();
    assert_eq!(
        forwarded("prepare"),
        [
            "prepare",
            "--manifest-path",
            &mirror("Cargo.toml"),
            "--package",
            "fixture",
            "--replicas",
            "0",
            "--output",
            &mirror("target/remote-prepared"),
            "--overlay",
            "false",
            "--tree",
            &format!("current={}", mirror("").trim_end_matches('/')),
            "--tree",
            &format!("other={}", mirror("").trim_end_matches('/')),
            "--scenario",
            "work",
            "--scenario",
            "alpha",
        ]
    );
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
            "--package",
            "fixture",
            "--rounds",
            "1",
            "--seed",
            "1",
            "--cpus",
            "0,2",
            "--timeout-seconds",
            "60",
            "--perf-event",
            "instructions",
            "--perf-event",
            "cycles",
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
    assert_eq!(
        forwarded("run"),
        [
            "run",
            "work",
            "--manifest-path",
            &mirror("Cargo.toml"),
            "--artifacts",
            &mirror("target/remote-prepared/artifacts.json"),
            "--mode",
            "screening",
            "--package",
            "fixture",
            "--rounds",
            "1",
            "--seed",
            "1",
            "--cpus",
            "0,2",
            "--timeout-seconds",
            "60",
            "--lock",
            "lock",
            "--output",
            &mirror("target/remote-run"),
            "--decision-rule",
            "rule",
            "--idle-host",
            "true",
            "--stack-padding",
            "false",
            "--perf-event",
            "instructions",
            "--perf-event",
            "cycles",
        ]
    );
    assert!(
        campaign["evidence"]
            .as_str()
            .unwrap()
            .starts_with("screening"),
        "the downloaded campaign is the template, a screening run: {campaign}"
    );
    assert_eq!(
        campaign["output"],
        root.join("target/remote-run").display().to_string()
    );

    // Smoke campaigns come back in declaration order, as a local smoke run returns them.
    let campaigns = call(
        &mut context,
        "benchmark:smoke",
        &[
            "--package",
            "fixture",
            "--lock",
            "lock",
            "--output",
            "target/remote-smoke",
        ],
    )
    .unwrap();
    assert_eq!(
        forwarded("smoke"),
        [
            "smoke",
            "--manifest-path",
            &mirror("Cargo.toml"),
            "--package",
            "fixture",
            "--output",
            &mirror("target/remote-smoke"),
            "--lock",
            "lock",
        ]
    );
    let outputs: Vec<_> = campaigns
        .as_array()
        .unwrap()
        .iter()
        .map(|campaign| campaign["output"].as_str().unwrap().to_owned())
        .collect();
    let local = |scenario: &str| {
        root.join("target/remote-smoke")
            .join(scenario)
            .display()
            .to_string()
    };
    assert_eq!(outputs, [local("work"), local("alpha")]);

    // A malformed CPU list fails before the host is contacted, as it does locally.
    let error = call(
        &mut context,
        "benchmark:run",
        &[
            "work",
            "--artifacts",
            "target/remote-prepared/artifacts.json",
            "--cpus",
            "x",
        ],
    )
    .unwrap_err();
    assert!(
        error.to_string().contains("benchmark:run: invalid CPU"),
        "{error}"
    );

    // So does a package that declares no scenarios.
    let error = call(&mut context, "benchmark:smoke", &["--package", "missing"]).unwrap_err();
    assert!(error.to_string().contains("missing"), "{error}");

    // A failure on the host is reported with the command that failed.
    let failing = script(&directory.path().join("failing"), "#!/bin/sh\nexit 3\n");
    let error = context
        .call(
            "benchmark:smoke",
            &[
                "--output",
                "target/remote-failed",
                "--host",
                "bench",
                "--remote-directory",
                host_text.as_str(),
                "--remote-program",
                failing.as_str(),
            ],
        )
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("base-benchmark smoke on bench failed"),
        "{error}"
    );
}
