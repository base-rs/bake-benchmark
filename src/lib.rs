// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! Bake tasks for the shared Base benchmark tooling.
//!
//! Each task translates its arguments into a [`base_benchmark::Workspace`] operation. Preparation, collection, locking, cleanup, validation, summaries, and every default are implemented once, in `base-benchmark`; the `base-benchmark` command calls the same operations.
use bake::{Context, Error, Result};
use base_benchmark::{
    ArtifactManifest, Campaign, Cancellation, Mode, PrepareRequest, Prepared, RunRequest, Suite,
    Tree, Workspace, parse_cpus, remote::Remote, summarize_directory,
};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

fn workspace(context: &Context, manifest_path: Option<PathBuf>) -> Workspace {
    Workspace::new(resolve(
        context,
        manifest_path.unwrap_or_else(|| PathBuf::from("Cargo.toml")),
    ))
}

/// Paths are relative to the project root; absolute paths are used as they are.
fn resolve(context: &Context, path: impl AsRef<Path>) -> PathBuf {
    context.root().join(path)
}

fn failed(error: std::io::Error) -> Error {
    Error::new(error.to_string())
}

/// The host a task runs on over SSH, when one is given; otherwise the task runs here.
fn remote(
    host: Option<String>,
    directory: Option<String>,
    program: Option<String>,
) -> Option<Remote> {
    host.map(|host| {
        let mut remote = Remote::new(host);
        remote.directory = directory.unwrap_or(remote.directory);
        remote.program = program;
        remote
    })
}

/// Command-line arguments for a remote command: each present option as `--name value`.
fn options(
    positional: &[&str],
    named: &[(&str, Option<String>)],
    repeated: &[(&str, &[String])],
) -> Vec<String> {
    let mut arguments: Vec<String> = positional.iter().map(|value| (*value).to_owned()).collect();
    for (name, value) in named {
        if let Some(value) = value {
            arguments.extend([format!("--{name}"), value.clone()]);
        }
    }
    for (name, values) in repeated {
        for value in *values {
            arguments.extend([format!("--{name}"), value.clone()]);
        }
    }
    arguments
}

fn display(path: Option<PathBuf>) -> Option<String> {
    path.map(|path| path.display().to_string())
}

/// Split `NAME=VALUE`, where neither may be empty, as the command does.
fn assignment<'a>(value: &'a str, form: &str) -> Result<(&'a str, &'a str)> {
    value
        .split_once('=')
        .filter(|(name, value)| !name.is_empty() && !value.is_empty())
        .ok_or_else(|| Error::new(format!("expected {form}, got {value:?}")))
}

/// Each `--program NAME=PATH`, with its path as given, refusing a name given twice, as the command does.
fn parse_programs(values: &[String]) -> Result<BTreeMap<&str, &str>> {
    let mut programs = BTreeMap::new();
    for value in values {
        let (name, path) = assignment(value, "NAME=PATH")?;
        if programs.insert(name, path).is_some() {
            return Err(Error::new(format!(
                "--program {name} was given more than once"
            )));
        }
    }
    Ok(programs)
}

/// The workspace with each external program, its path resolved against the project root, since it is a path on this host.
fn with_programs(
    context: &Context,
    mut workspace: Workspace,
    programs: BTreeMap<&str, &str>,
) -> Workspace {
    for (name, path) in programs {
        workspace = workspace.with_program(name, resolve(context, path));
    }
    workspace
}

/// Each `--role-cpus ROLE=CPUS` as the command reads it.
fn parse_role_cpus(values: &[String]) -> Result<BTreeMap<String, Vec<usize>>> {
    values
        .iter()
        .map(|value| {
            let (role, cpus) = assignment(value, "ROLE=CPUS")?;
            Ok((role.to_owned(), parse_cpus(cpus).map_err(failed)?))
        })
        .collect()
}

/// List workspace packages that declare `[package.metadata.base-benchmark]` scenarios, including unpublished packages, with resolved scenario defaults. Nothing is built or run.
#[bake::task]
pub fn list(
    context: &mut Context,
    #[bake(help = "Workspace manifest; defaults to Cargo.toml in the project root.")] manifest_path: Option<PathBuf>,
) -> Result<Vec<Suite>> {
    workspace(context, manifest_path).suites().map_err(failed)
}

/// Build labelled source trees with identical settings and freeze their executables. The first tree is the baseline; without trees, the current workspace is built.
#[bake::task]
#[allow(clippy::too_many_arguments, reason = "each argument is a task option")]
pub fn prepare(
    context: &mut Context,
    #[bake(help = "Benchmark package; required when several declare scenarios.")] package: Option<
        String,
    >,
    #[bake(help = "LABEL=PATH source tree; repeat for each tree, baseline first.")] tree: Vec<Tree>,
    #[bake(
        help = "Layout replicas of the baseline to rebuild at other paths; defaults to 1 for two or more trees, otherwise 0."
    )]
    replicas: Option<usize>,
    #[bake(help = "New output directory; defaults to target/base-benchmark/prepared-TIME.")] output: Option<PathBuf>,
    #[bake(help = "Prepare only the feature sets this scenario needs; repeat for each.")] scenario: Vec<String>,
    #[bake(
        named,
        default = false,
        help = "Build every tree with this workspace's workload package."
    )]
    overlay: bool,
    #[bake(help = "Workspace manifest; defaults to Cargo.toml in the project root.")] manifest_path: Option<PathBuf>,
    #[bake(help = "Prepare on this SSH host, mirroring local paths there.")] host: Option<String>,
    #[bake(
        help = "The host's directory for mirrors and the command; defaults to .cache/base-benchmark."
    )]
    remote_directory: Option<String>,
    #[bake(
        help = "The host's base-benchmark command; without it, the crates.io release of this version is installed, so unreleased builds need one."
    )]
    remote_program: Option<String>,
) -> Result<Prepared> {
    let tree: Vec<Tree> = tree
        .into_iter()
        .map(|tree| Tree {
            path: resolve(context, tree.path),
            ..tree
        })
        .collect();
    let output = output.map(|output| resolve(context, output));
    let manifest_path = resolve(
        context,
        manifest_path.unwrap_or_else(|| PathBuf::from("Cargo.toml")),
    );
    if let Some(remote) = remote(host, remote_directory, remote_program) {
        // A termination signal cancels the host's command, whose output is still copied back.
        let cancellation = Cancellation::new();
        let _signals = cancellation.on_termination_signals();
        let trees: Vec<String> = tree
            .iter()
            .map(|tree| format!("{}={}", tree.label, tree.path.display()))
            .collect();
        return remote
            .execute(
                "prepare",
                options(
                    &[],
                    &[
                        ("manifest-path", display(Some(manifest_path))),
                        ("package", package),
                        ("replicas", replicas.map(|replicas| replicas.to_string())),
                        ("output", display(output)),
                        ("overlay", Some(overlay.to_string())),
                    ],
                    &[("tree", &trees), ("scenario", &scenario)],
                ),
                &cancellation,
            )
            .and_then(|outcome| {
                // The downloaded manifest names the host's executables, for runs on the same host.
                let manifest_path = outcome.output.join("artifacts.json");
                ArtifactManifest::read(&manifest_path).map(|manifest| Prepared {
                    manifest_path,
                    manifest,
                })
            })
            .map_err(failed);
    }
    let cancellation = Cancellation::new();
    let _signals = cancellation.on_termination_signals();
    Workspace::new(manifest_path)
        .prepare(
            PrepareRequest {
                package,
                trees: tree,
                replicas,
                output,
                scenarios: scenario,
                overlay,
            },
            &cancellation,
        )
        .map_err(failed)
}

/// Build the current workspace and run every scenario once with its smoke arguments. Smoke results are not performance evidence.
#[bake::task]
#[allow(clippy::too_many_arguments, reason = "each argument is a task option")]
pub fn smoke(
    context: &mut Context,
    #[bake(help = "Benchmark package; required when several declare scenarios.")] package: Option<
        String,
    >,
    #[bake(help = "New output directory; defaults to target/base-benchmark/smoke-TIME.")]
    output: Option<PathBuf>,
    #[bake(
        help = "Host-wide lock file; defaults to /tmp/base-benchmark-host.lock. With --host, a path on the host, relative to the remote user's home."
    )]
    lock: Option<PathBuf>,
    #[bake(
        help = "NAME=PATH of an external program that a scenario's role runs; repeat for each. With --host, a path on the host, relative to the remote user's home."
    )]
    program: Vec<String>,
    #[bake(help = "Workspace manifest; defaults to Cargo.toml in the project root.")] manifest_path: Option<PathBuf>,
    #[bake(help = "Smoke on this SSH host, mirroring local paths there.")] host: Option<String>,
    #[bake(
        help = "The host's directory for mirrors and the command; defaults to .cache/base-benchmark."
    )]
    remote_directory: Option<String>,
    #[bake(
        help = "The host's base-benchmark command; without it, the crates.io release of this version is installed, so unreleased builds need one."
    )]
    remote_program: Option<String>,
) -> Result<Vec<Campaign>> {
    let output = output.map(|output| resolve(context, output));
    // Checked here, so that a remote smoke run fails before contacting the host, as a local one does.
    let programs = parse_programs(&program)?;
    if let Some(remote) = remote(host, remote_directory, remote_program) {
        // A termination signal cancels the host's command, whose output is still copied back.
        let cancellation = Cancellation::new();
        let _signals = cancellation.on_termination_signals();
        let manifest_path = resolve(
            context,
            manifest_path.unwrap_or_else(|| PathBuf::from("Cargo.toml")),
        );
        let scenarios: Vec<String> = Workspace::new(manifest_path.clone())
            .suite(package.as_deref())
            .map_err(failed)?
            .scenarios
            .into_iter()
            .map(|scenario| scenario.name)
            .collect();
        return remote
            .execute(
                "smoke",
                options(
                    &[],
                    &[
                        ("manifest-path", display(Some(manifest_path))),
                        ("package", package),
                        ("output", display(output)),
                        // The lock is the host's, named as it is.
                        ("lock", display(lock)),
                    ],
                    // So are external programs.
                    &[("program", &program)],
                ),
                &cancellation,
            )
            .and_then(|outcome| {
                // Each scenario's campaign is named after it, in declaration order, as a local smoke run returns them.
                scenarios
                    .iter()
                    .map(|scenario| summarize_directory(&outcome.output.join(scenario)))
                    .collect::<std::io::Result<Vec<_>>>()
            })
            .map(report_all)
            .map_err(failed);
    }
    let cancellation = Cancellation::new();
    let _signals = cancellation.on_termination_signals();
    let lock = lock.map(|lock| resolve(context, lock));
    let campaigns = with_programs(context, workspace(context, manifest_path), programs)
        .smoke(
            package.as_deref(),
            output,
            lock,
            &cancellation,
            &mut std::io::stderr(),
        )
        .map_err(failed)?;
    Ok(report_all(campaigns))
}

/// Write each campaign's directory and report to stderr, as every collecting task does.
fn report_all(campaigns: Vec<Campaign>) -> Vec<Campaign> {
    for campaign in &campaigns {
        eprintln!("{}\n{}", campaign.output.display(), campaign.report);
    }
    campaigns
}

/// Collect a scenario from prepared artifacts in seeded random rounds under the host lock. The report is written to stderr; the result describes the campaign directory, evidence class, and warnings.
#[bake::task]
#[allow(clippy::too_many_arguments, reason = "each argument is a task option")]
pub fn run(
    context: &mut Context,
    #[bake(help = "Scenario name.")] scenario: String,
    #[bake(named, help = "artifacts.json from benchmark:prepare.")] artifacts: PathBuf,
    #[bake(help = "smoke, screening, or recorded; defaults to screening.")] mode: Option<Mode>,
    #[bake(help = "Benchmark package; required when several declare scenarios.")] package: Option<
        String,
    >,
    #[bake(help = "Rounds; defaults to 1, 5, or 10 by mode.")] rounds: Option<u32>,
    #[bake(
        help = "Seed for the round order and stack padding; defaults to the clock and is recorded."
    )]
    seed: Option<u64>,
    #[bake(
        help = "CPU list for every workload process of a single-process scenario, such as 2 or 2,4-5. Required for recorded runs."
    )]
    cpus: Option<String>,
    #[bake(
        help = "ROLE=CPUS, such as server=2,3, pinning a multi-process scenario's role; repeat for each role. Recorded runs need every role placed."
    )]
    role_cpus: Vec<String>,
    #[bake(
        help = "NAME=PATH of an external program that a scenario's role runs; repeat for each. With --host, a path on the host, relative to the remote user's home."
    )]
    program: Vec<String>,
    #[bake(
        help = "Whole-process perf stat event for a single-process scenario; repeat for several."
    )]
    perf_event: Vec<String>,
    #[bake(
        help = "Seconds before a workload process, or a multi-process scenario's session, is killed; defaults to 300."
    )]
    timeout_seconds: Option<u64>,
    #[bake(
        help = "Host-wide lock file; defaults to /tmp/base-benchmark-host.lock. With --host, a path on the host, relative to the remote user's home."
    )]
    lock: Option<PathBuf>,
    #[bake(help = "New output directory; defaults to target/base-benchmark/SCENARIO-MODE-TIME.")]
    output: Option<PathBuf>,
    #[bake(help = "The predeclared interpretation rule. Required for recorded runs.")]
    decision_rule: Option<String>,
    #[bake(
        named,
        default = false,
        help = "Declare that the host is otherwise idle. Required for recorded runs."
    )]
    idle_host: bool,
    #[bake(help = "Vary initial stack alignment per sample; defaults to true.")]
    stack_padding: Option<bool>,
    #[bake(help = "Workspace manifest; defaults to Cargo.toml in the project root.")] manifest_path: Option<PathBuf>,
    #[bake(help = "Collect on this SSH host, from artifacts prepared there.")] host: Option<String>,
    #[bake(
        help = "The host's directory for mirrors and the command; defaults to .cache/base-benchmark."
    )]
    remote_directory: Option<String>,
    #[bake(
        help = "The host's base-benchmark command; without it, the crates.io release of this version is installed, so unreleased builds need one."
    )]
    remote_program: Option<String>,
) -> Result<Campaign> {
    // Checked here, in the command's order, so that a remote run fails before contacting the host, as a local one does.
    let programs = parse_programs(&program)?;
    let placement = cpus
        .as_deref()
        .map(parse_cpus)
        .transpose()
        .map_err(failed)?
        .unwrap_or_default();
    let role_placement = parse_role_cpus(&role_cpus)?;
    if let Some(remote) = remote(host, remote_directory, remote_program) {
        // A termination signal cancels the host's command, whose output is still copied back.
        let cancellation = Cancellation::new();
        let _signals = cancellation.on_termination_signals();
        let manifest_path = resolve(
            context,
            manifest_path.unwrap_or_else(|| PathBuf::from("Cargo.toml")),
        );
        return remote
            .execute(
                "run",
                options(
                    &[&scenario],
                    &[
                        ("manifest-path", display(Some(manifest_path))),
                        ("artifacts", display(Some(resolve(context, artifacts)))),
                        ("mode", mode.map(|mode| mode.to_string())),
                        ("package", package),
                        ("rounds", rounds.map(|rounds| rounds.to_string())),
                        ("seed", seed.map(|seed| seed.to_string())),
                        ("cpus", cpus),
                        (
                            "timeout-seconds",
                            timeout_seconds.map(|seconds| seconds.to_string()),
                        ),
                        // The lock is the host's, named as it is.
                        ("lock", display(lock)),
                        (
                            "output",
                            display(output.map(|output| resolve(context, output))),
                        ),
                        ("decision-rule", decision_rule),
                        ("idle-host", Some(idle_host.to_string())),
                        (
                            "stack-padding",
                            stack_padding.map(|padding| padding.to_string()),
                        ),
                    ],
                    // External programs are the host's, named as they are.
                    &[
                        ("role-cpus", &role_cpus),
                        ("program", &program),
                        ("perf-event", &perf_event),
                    ],
                ),
                &cancellation,
            )
            .and_then(|outcome| summarize_directory(&outcome.output))
            .map(|campaign| report_all(vec![campaign]).remove(0))
            .map_err(failed);
    }
    let request = RunRequest {
        package,
        mode,
        rounds,
        seed,
        cpus: placement,
        role_cpus: role_placement,
        perf_events: perf_event,
        timeout: timeout_seconds.map(Duration::from_secs),
        lock: lock.map(|lock| resolve(context, lock)),
        output: output.map(|output| resolve(context, output)),
        decision_rule,
        idle_host,
        stack_padding,
        invocation: std::env::args().collect(),
        ..RunRequest::new(scenario, resolve(context, artifacts))
    };
    let cancellation = Cancellation::new();
    let _signals = cancellation.on_termination_signals();
    let campaign = with_programs(context, workspace(context, manifest_path), programs)
        .run(request, &cancellation, &mut std::io::stderr())
        .map_err(failed)?;
    eprintln!("{}\n{}", campaign.output.display(), campaign.report);
    Ok(campaign)
}

/// Regenerate `summary.json` from the raw samples of a complete campaign.
#[bake::task]
pub fn summarize(
    context: &mut Context,
    #[bake(help = "Campaign directory.")] output: PathBuf,
) -> Result<Campaign> {
    let campaign = summarize_directory(&resolve(context, output)).map_err(failed)?;
    eprintln!("{}", campaign.report);
    Ok(campaign)
}
