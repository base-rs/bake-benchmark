# Bake Integration

## Tasks

| Task | Core operation |
| --- | --- |
| `benchmark:list` | `Workspace::suites`: packages with `[package.metadata.base-benchmark]`, including unpublished packages. Nothing is built or run. |
| `benchmark:prepare` | `Workspace::prepare`: labelled `--tree LABEL=PATH` source trees, baseline first, plus `--replicas N` layout replicas of the baseline (one by default for two or more trees). Writes `artifacts.json`. |
| `benchmark:smoke` | `Workspace::smoke`: build the current tree and run every scenario once with its smoke arguments. |
| `benchmark:run` | `Workspace::run`: collect a scenario from prepared artifacts in `screening` or `recorded` mode. |
| `benchmark:summarize` | `summarize_directory`: regenerate `summary.json` from a complete campaign. |

The planned `benchmark:build` became `benchmark:prepare`, because the operation also freezes executables, records their identity, and builds the layout replica. Relative paths are resolved against the Bake project root. Absolute paths are used as they are, and `--manifest-path` selects a standalone workspace, such as a benchmark suite excluded from the main workspace. `prepare`, `run`, and `smoke` cancel on the first SIGINT, SIGTERM, or SIGHUP, so builds stop between trees and workload process groups are killed before the host lock is released; a second signal exits immediately. Tasks pass unset options through, and the core resolves every default, so a task and the `base-benchmark` command given equivalent options produce the same plan, summary, and errors.

Registration uses ordinary `#[bake::task]` attributes and the crate-derived `benchmark:` namespace; there is no plugin registry. Discovery uses locked Cargo metadata rather than `bake_cargo::packages`, which deliberately filters for publishable packages. Historical `results/**/Cargo.toml` snapshots are never executed.

## Dependency

`bake-benchmark` depends on `base-benchmark`, never the reverse. During initial development the dependency is a sibling path, `../base-benchmark`. Before this crate is merged with working CI or published, switch it to `base-benchmark`'s Git repository or a crates.io release.

Consumers add this crate to their private `bake/Cargo.toml`. It is never a dependency of workloads or published libraries. Integration through `base-project` should follow demonstrated use in more than one project.

## Not yet covered

- Task options for capabilities the core does not yet implement: multi-process roles, downstream patch preparation, revision archives, and external comparator subjects.
- History import, reporting, or a comparison UI; see the core's [history follow-up](https://github.com/base-rs/base-benchmark/blob/main/context/history.md).
- Investigation record generation. Use the core's [investigation convention](https://github.com/base-rs/base-benchmark/blob/main/context/investigations.md) and template by hand.

Later entry points must delegate to shared core APIs rather than add a Bake-only runner, store, or statistics pipeline.
