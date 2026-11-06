# Thin Bake Adapter

## Requirement

Keep `bake-benchmark` in its own repository as a Rust adapter to the Bake-independent `base-benchmark`. Task entry points must delegate shared measurement, collection, record validation, provenance, cleanup, persistence, and summary semantics to the core rather than implement a second runner. Normal workflows must not require Python or merely wrap historical Python scripts.

Do not force workload binaries or measured public libraries to depend on Bake. Keep project-specific workloads, correctness oracles, protocol semantics, operation boundaries, and performance budgets in the consuming projects. Do not present proposed tasks as implemented capabilities.

## Rationale

The user explicitly selected the two-repository split on 6 November 2026 and requested unified tooling without Python. The [audit](https://github.com/base-rs/base-benchmark/blob/main/reports/benchmarking-landscape.md) found reusable mechanics but different measurement semantics, plus a concrete discovery pitfall for unpublished workload packages.

## Scope

The published adapter, its dependency on the core, task registration/configuration, and documentation. The private `bake/` package is repository development automation and does not link the benchmark tasks.

## Validation

Inspect `cargo tree` for the adapter → core direction and the task functions for logic beyond argument translation, path resolution, and signal-guarded calls into `Workspace`. Task tests discover the registry and invoke every task by name against a fixture workspace, including an unpublished package. They also check that errors come from the shared implementation, compare the `base-benchmark` command and the tasks under equivalent plans for identical summaries and errors, and test an explicitly configured standalone manifest selected with `--manifest-path`. Normal paths run without Python.
