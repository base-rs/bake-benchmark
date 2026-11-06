# Releases

## Unreleased

- Establish the repository and shared Base development tooling.
- Record Bake-specific audit findings and the thin-adapter design.
- Add `benchmark:list`, `benchmark:prepare`, `benchmark:smoke`, `benchmark:run`, and `benchmark:summarize`, each delegating to `base-benchmark`'s `Workspace` operations, with compact results and signal cancellation.
- Leave every default to the core, add `--replicas`, `--stack-padding`, and `--manifest-path` on every task, resolve relative paths against the project root, and test command and task equivalence, standalone manifests, absolute paths, recorded-mode errors, and perf event passthrough.
