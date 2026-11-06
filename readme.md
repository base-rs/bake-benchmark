# `bake-benchmark`

Thin Rust Bake integration for [`base-benchmark`](https://github.com/base-rs/base-benchmark).

**Status: planning scaffold, not a working task provider.** No `benchmark:*` tasks, cross-crate API integration, crates.io release, or downstream migration exists yet. The first release awaits implementation and review; repository setup does not publish a crate.

## Findings and direction

- [Bake-specific findings and proposed tasks](reports/bake-integration.md).
- [Full organization-wide audit](https://github.com/base-rs/base-benchmark/blob/main/reports/benchmarking-landscape.md).
- [Integration plan](context/integration.md) and [architectural requirements](design/integration.md).

The dependency direction will be `bake-benchmark` → `base-benchmark`. The latter owns records, measurement, collection, process supervision, validation, provenance, and statistics. This crate translates Bake configuration/tasks into those APIs; it must not maintain a second runner or require Python for normal workflows. Workloads and correctness checks remain project-specific.

## Development

Install the [Bake launcher](https://github.com/base-rs/bake), then run `cargo bake test`. The private `bake/` package supplies shared Base development tasks. These development tasks are not the proposed benchmark task family. Cross-repository dependencies will be added when the core exposes an API to consume; the empty library intentionally has no fake integration.

## Releases

See [releases.md](releases.md). No releases yet.

## Contributing

Open an issue or pull request on [GitHub](https://github.com/base-rs/bake-benchmark).

### Agent Context

Run `cargo bake agent:context:install` to install shared context and skills, then inspect `.agents/context/index.md`. Follow `.agents/implementation.md`, discover `design/*.md` before implementation, and use the installed design review skills.
