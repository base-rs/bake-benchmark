# `bake-benchmark`

Bake tasks for the shared Base benchmark tooling in [`base-benchmark`](https://github.com/base-rs/base-benchmark).

The tasks are `benchmark:list`, `benchmark:prepare`, `benchmark:smoke`, `benchmark:run`, and `benchmark:summarize`. Each one translates its arguments into the same `base-benchmark` operation that the `base-benchmark` command uses. Preparation, collection, locking, cleanup, validation, and summaries are implemented once, in the core. No Python is involved.

**Status:** an initial implementation on `base-benchmark` 0.4, used by the `base-html` rendering benchmark. The tasks expose the core's multi-process scenarios: `benchmark:smoke` and `benchmark:run` take their external programs with `--program NAME=PATH`, and `benchmark:run` places their roles with `--role-cpus ROLE=CPUS`.

## Usage

Add the tasks to a project's private Bake package:

```sh
cargo add --manifest-path bake/Cargo.toml bake-benchmark
cargo bake --regenerate
cargo bake benchmark:list
cargo bake benchmark:smoke
```

See [Using the Shared Benchmark Tooling](https://github.com/base-rs/base-benchmark/blob/main/context/usage.md) for workloads, scenario configuration, and interpreting results, and run `cargo bake benchmark:run --help` in a project that uses the tasks for their options. Defaults, such as the mode, round count, timeout, lock path, and stack padding, are resolved by the core, so the command and the tasks behave the same. Tasks return a compact JSON result (the campaign directory, summary path, evidence class, and warnings) and write progress and the human report to stderr.

## Design and findings

- [Integration](context/integration.md) and [requirements](design/integration.md).
- [Bake-specific audit findings](reports/bake-integration.md).

## Development

Install the [Bake launcher](https://github.com/base-rs/bake), then run `cargo bake test`. Task tests discover the registry and invoke each task by name against a fixture workspace. The private `bake/` package supplies shared Base development tasks and does not include the benchmark tasks.

## Releases

See [releases.md](releases.md).

## Contributing

Open an issue or pull request on [GitHub](https://github.com/base-rs/bake-benchmark).

### Agent Context

Run `cargo bake agent:context:install` to install shared context and skills, then inspect `.agents/context/index.md`. Follow `.agents/implementation.md`, discover `design/*.md` before implementation, and use the installed design review skills.
