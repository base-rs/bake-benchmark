# Releases

## Unreleased

- Breaking: update to `base-benchmark` 0.3, whose types the tasks return; see its [release notes](https://github.com/base-rs/base-benchmark/blob/main/releases.md#v030). `Summary`, reached through `Campaign::summary`, and the summary types within it are `#[non_exhaustive]`, and `Summary` gains `overlays`, so the reports and `summary.json` of overlaid campaigns state the overlaid package and each artifact's lockfile changes. The `warnings` the tasks return name every metric with the same hazard in one warning, build layout first. `benchmark:prepare` signalled while a build runs fails as cancelled and writes no manifest, rather than succeeding once Cargo finishes the build, and reads each tree's Cargo metadata again with network access when reading it offline fails, rather than recording its path dependencies as unknown. With `--host`, a task reports a program that fails once it is cancelled as cancelled, and a host command that fails before creating its output with the host's error alone.

## v0.2.0

- Breaking: update to `base-benchmark` 0.2, whose types the tasks return. `benchmark:prepare` takes `--scenario`, repeatable, to prepare only the feature sets those scenarios need, and `--overlay true` to build every tree with this workspace's workload package.
- Add `--host`, `--remote-directory`, and `--remote-program` to `benchmark:prepare`, `benchmark:run`, and `benchmark:smoke`, to run them on a measuring host over SSH and return the downloaded results.

## v0.1.0

- Establish the repository and shared Base development tooling.
- Record Bake-specific audit findings and the thin-adapter design.
- Add `benchmark:list`, `benchmark:prepare`, `benchmark:smoke`, `benchmark:run`, and `benchmark:summarize`, each delegating to `base-benchmark`'s `Workspace` operations, with compact results and signal cancellation.
- Leave every default to the core, add `--replicas`, `--stack-padding`, and `--manifest-path` on every task, resolve relative paths against the project root, and test command and task equivalence, standalone manifests, absolute paths, recorded-mode errors, and perf event passthrough.
