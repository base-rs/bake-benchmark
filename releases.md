# Releases

## Unreleased

- Breaking: update to `base-benchmark` 0.2, whose types the tasks return. `benchmark:prepare` takes `--scenario`, repeatable, to prepare only the feature sets those scenarios need, and `--overlay true` to build every tree with this workspace's workload package.
- Add `--host`, `--remote-directory`, and `--remote-program` to `benchmark:prepare`, `benchmark:run`, and `benchmark:smoke`, to run them on a measuring host over SSH and return the downloaded results.

## v0.1.0

- Establish the repository and shared Base development tooling.
- Record Bake-specific audit findings and the thin-adapter design.
- Add `benchmark:list`, `benchmark:prepare`, `benchmark:smoke`, `benchmark:run`, and `benchmark:summarize`, each delegating to `base-benchmark`'s `Workspace` operations, with compact results and signal cancellation.
- Leave every default to the core, add `--replicas`, `--stack-padding`, and `--manifest-path` on every task, resolve relative paths against the project root, and test command and task equivalence, standalone manifests, absolute paths, recorded-mode errors, and perf event passthrough.
