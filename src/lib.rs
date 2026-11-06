// Released under the MIT License.
// Copyright, 2026, by Samuel Williams.

//! Bake tasks for the shared Base benchmark tooling.
//!
//! Each task translates its arguments into a [`base_benchmark::Workspace`] operation. Preparation, collection, locking, cleanup, validation, summaries, and every default are implemented once, in `base-benchmark`; the `base-benchmark` command calls the same operations.
use bake::{Context, Error, Result};
use base_benchmark::{
    Campaign, Cancellation, Mode, Prepared, RunRequest, Suite, Tree, Workspace, parse_cpus,
    summarize_directory,
};
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
    #[bake(help = "Workspace manifest; defaults to Cargo.toml in the project root.")] manifest_path: Option<PathBuf>,
) -> Result<Prepared> {
    let tree = tree
        .into_iter()
        .map(|tree| Tree {
            path: resolve(context, tree.path),
            ..tree
        })
        .collect();
    let output = output.map(|output| resolve(context, output));
    let cancellation = Cancellation::new();
    let _signals = cancellation.on_termination_signals();
    workspace(context, manifest_path)
        .prepare(package.as_deref(), tree, replicas, output, &cancellation)
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
    #[bake(help = "Host-wide lock file; defaults to /tmp/base-benchmark-host.lock.")] lock: Option<
        PathBuf,
    >,
    #[bake(help = "Workspace manifest; defaults to Cargo.toml in the project root.")] manifest_path: Option<PathBuf>,
) -> Result<Vec<Campaign>> {
    let cancellation = Cancellation::new();
    let _signals = cancellation.on_termination_signals();
    let lock = lock.map(|lock| resolve(context, lock));
    let output = output.map(|output| resolve(context, output));
    let campaigns = workspace(context, manifest_path)
        .smoke(
            package.as_deref(),
            output,
            lock,
            &cancellation,
            &mut std::io::stderr(),
        )
        .map_err(failed)?;
    for campaign in &campaigns {
        eprintln!("{}\n{}", campaign.output.display(), campaign.report);
    }
    Ok(campaigns)
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
        help = "CPU list for every workload process, such as 2 or 2,4-5. Required for recorded runs."
    )]
    cpus: Option<String>,
    #[bake(help = "Whole-process perf stat event; repeat for several.")] perf_event: Vec<String>,
    #[bake(help = "Seconds before a workload process is killed; defaults to 300.")] timeout_seconds: Option<u64>,
    #[bake(help = "Host-wide lock file; defaults to /tmp/base-benchmark-host.lock.")] lock: Option<
        PathBuf,
    >,
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
) -> Result<Campaign> {
    let request = RunRequest {
        package,
        mode,
        rounds,
        seed,
        cpus: cpus
            .as_deref()
            .map(parse_cpus)
            .transpose()
            .map_err(failed)?
            .unwrap_or_default(),
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
    let campaign = workspace(context, manifest_path)
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
