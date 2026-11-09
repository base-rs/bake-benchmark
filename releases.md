# Releases

## v0.3.0

- Breaking: update to `base-benchmark` 0.4, whose types the tasks return; see its [release notes](https://github.com/base-rs/base-benchmark/blob/main/releases.md#v040). It adds multi-process scenarios: services and one driver per sample, ready lines, the driver's window markers, and services' CPU per request. Prepared manifests are version 3, plans version 3, campaigns version 2, and summaries version 3. `ArtifactManifest`, reached through `Prepared::manifest`, gains `helpers`, the helper roles built once from the baseline tree, and `Summary`, reached through `Campaign::summary`, gains `placement`, while its comparisons involving invalid responses are invalid, without metrics (`PairSummary::invalid`). The scenarios of each `Suite` that `benchmark:list` returns gain `roles` and `shared_placement`. `benchmark:smoke` on a workspace whose roles run external programs fails unless `--program` gives each one's path, since 0.4's `Workspace::smoke` needs the program of every scenario's roles. The `warnings` the tasks return include roles that share physical cores and, first, invalid comparisons.
- Add `--program NAME=PATH`, repeatable, to `benchmark:smoke` and `benchmark:run`, giving the path of each external program that a scenario's role runs, and `--role-cpus ROLE=CPUS`, repeatable, to `benchmark:run`, pinning each role to its own CPUs. Program paths are resolved against the project root, or with `--host` forwarded unchanged as paths on the host, like `--lock`. Malformed values, and a program named twice, fail with the command's errors, before the host is contacted.
- Breaking: update to `base-benchmark` 0.3, whose types the tasks return; see its [release notes](https://github.com/base-rs/base-benchmark/blob/main/releases.md#v030). `Summary`, reached through `Campaign::summary`, and the summary types within it are `#[non_exhaustive]`, and `Summary` gains `overlays`, so the reports and `summary.json` of overlaid campaigns state the overlaid package and each artifact's lockfile changes. The `warnings` the tasks return name every metric with the same hazard in one warning, build layout first. `benchmark:prepare` signalled while a build runs fails as cancelled and writes no manifest, rather than succeeding once Cargo finishes the build, and reads each tree's Cargo metadata again with network access when reading it offline fails, rather than recording its path dependencies as unknown. With `--host`, a task reports a program that fails once it is cancelled as cancelled, and a host command that fails before creating its output with the host's error alone.

## v0.2.0

- Breaking: update to `base-benchmark` 0.2, whose types the tasks return. `benchmark:prepare` takes `--scenario`, repeatable, to prepare only the feature sets those scenarios need, and `--overlay true` to build every tree with this workspace's workload package.
- Add `--host`, `--remote-directory`, and `--remote-program` to `benchmark:prepare`, `benchmark:run`, and `benchmark:smoke`, to run them on a measuring host over SSH and return the downloaded results.

## v0.1.0

- Establish the repository and shared Base development tooling.
- Record Bake-specific audit findings and the thin-adapter design.
- Add `benchmark:list`, `benchmark:prepare`, `benchmark:smoke`, `benchmark:run`, and `benchmark:summarize`, each delegating to `base-benchmark`'s `Workspace` operations, with compact results and signal cancellation.
- Leave every default to the core, add `--replicas`, `--stack-padding`, and `--manifest-path` on every task, resolve relative paths against the project root, and test command and task equivalence, standalone manifests, absolute paths, recorded-mode errors, and perf event passthrough.
