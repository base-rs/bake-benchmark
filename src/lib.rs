// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! Bake tasks for the shared Base benchmark tooling.
//!
//! Each task translates its arguments into a [`base_benchmark::Workspace`] operation. Preparation, collection, locking, cleanup, validation, and summaries are implemented once, in `base-benchmark`; the `base-benchmark` command calls the same operations.

use bake::{Context, Error, Result};
use base_benchmark::{
    Campaign, Cancellation, Mode, Prepared, RunRequest, Suite, Tree, Workspace, default_lock_path,
    parse_cpus, summarize_directory,
};
use std::path::{Path, PathBuf};
use std::time::Duration;

fn workspace(context: &Context, manifest_path: &Path) -> Workspace {
    Workspace::new(context.root().join(manifest_path))
}

fn failed(error: std::io::Error) -> Error {
    Error::new(error.to_string())
}

/// List workspace packages that declare `[package.metadata.base-benchmark]` scenarios, including unpublished packages. Nothing is built or run.
#[bake::task]
pub fn list(
    context: &mut Context,
    #[bake(
        default = "Cargo.toml",
        help = "Workspace manifest, relative to the project root."
    )]
    manifest_path: PathBuf,
) -> Result<Vec<Suite>> {
    workspace(context, &manifest_path).suites().map_err(failed)
}

/// Build labelled source trees with identical settings and freeze their executables. The first tree is the baseline; without trees, the current workspace is built.
#[bake::task]
pub fn prepare(
    context: &mut Context,
    #[bake(help = "Benchmark package; required when several declare scenarios.")] package: Option<
        String,
    >,
    #[bake(help = "LABEL=PATH source tree; repeat for each tree, baseline first.")] tree: Vec<Tree>,
    #[bake(
        help = "Rebuild a copy of the baseline at another path; defaults to true for two or more trees."
    )]
    replica: Option<bool>,
    #[bake(help = "New output directory; defaults to target/base-benchmark/prepared-TIME.")] output: Option<PathBuf>,
    #[bake(
        default = "Cargo.toml",
        help = "Workspace manifest, relative to the project root."
    )]
    manifest_path: PathBuf,
) -> Result<Prepared> {
    let tree = tree
        .into_iter()
        .map(|tree| Tree {
            path: context.root().join(tree.path),
            ..tree
        })
        .collect();
    let output = output.map(|output| context.root().join(output));
    workspace(context, &manifest_path)
        .prepare(package.as_deref(), tree, replica, output)
        .map_err(failed)
}

/// Build the current workspace and run every scenario once with its smoke arguments. Smoke results are not performance evidence.
#[bake::task]
pub fn smoke(
    context: &mut Context,
    #[bake(help = "Benchmark package; required when several declare scenarios.")] package: Option<
        String,
    >,
    #[bake(help = "New output directory; defaults to target/base-benchmark/smoke-TIME.")]
    output: Option<PathBuf>,
    #[bake(help = "Host-wide lock file; defaults to the shared lock in the temporary directory.")]
    lock: Option<PathBuf>,
    #[bake(
        default = "Cargo.toml",
        help = "Workspace manifest, relative to the project root."
    )]
    manifest_path: PathBuf,
) -> Result<Vec<Campaign>> {
    let cancellation = Cancellation::new();
    let _signals = cancellation.on_termination_signals();
    let lock = lock
        .map(|lock| context.root().join(lock))
        .unwrap_or_else(default_lock_path);
    let output = output.map(|output| context.root().join(output));
    let campaigns = workspace(context, &manifest_path)
        .smoke(
            package.as_deref(),
            output,
            &lock,
            &cancellation,
            &mut std::io::stderr(),
        )
        .map_err(failed)?;
    for campaign in &campaigns {
        eprintln!("{}\n{}", campaign.output.display(), campaign.report);
    }
    Ok(campaigns)
}

/// Collect a scenario from prepared artifacts in seeded random rounds under the host lock. The report is written to stderr; the result describes the campaign directory and summary.
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
    #[bake(help = "Seed for the round order; defaults to the clock and is recorded.")] seed: Option<
        u64,
    >,
    #[bake(
        help = "CPU list for every workload process, such as 2 or 2,4-5. Required for recorded runs."
    )]
    cpus: Option<String>,
    #[bake(help = "Whole-process perf stat event; repeat for several.")] perf_event: Vec<String>,
    #[bake(default = 300, help = "Seconds before a workload process is killed.")]
    timeout_seconds: u64,
    #[bake(help = "Host-wide lock file; defaults to the shared lock in the temporary directory.")]
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
    #[bake(
        default = "Cargo.toml",
        help = "Workspace manifest, relative to the project root."
    )]
    manifest_path: PathBuf,
) -> Result<Campaign> {
    let mut request = RunRequest::new(
        scenario,
        context.root().join(artifacts),
        mode.unwrap_or(Mode::Screening),
    );
    request.package = package;
    request.rounds = rounds;
    request.seed = seed;
    request.cpus = cpus
        .as_deref()
        .map(parse_cpus)
        .transpose()
        .map_err(failed)?
        .unwrap_or_default();
    request.perf_events = perf_event;
    request.timeout = Duration::from_secs(timeout_seconds);
    request.lock = lock
        .map(|lock| context.root().join(lock))
        .unwrap_or_else(default_lock_path);
    request.output = output.map(|output| context.root().join(output));
    request.decision_rule = decision_rule;
    request.idle_host = idle_host;
    request.invocation = std::env::args().collect();
    let cancellation = Cancellation::new();
    let _signals = cancellation.on_termination_signals();
    let campaign = workspace(context, &manifest_path)
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
    let campaign = summarize_directory(&context.root().join(output)).map_err(failed)?;
    eprintln!("{}", campaign.report);
    Ok(campaign)
}
