# Bake Integration Plan

This is a proposal, not an implemented task provider. Add a real dependency on `base-benchmark` when a usable core API exists; do not introduce a sibling path dependency or fake no-op tasks merely to connect the initial scaffolds.

## Proposed tasks

- `benchmark:list`: list explicitly configured packages/targets/scenarios without running workload code.
- `benchmark:build`: delegate shared preparation of labelled trees/archives and optional downstream patch-override workloads, with common explicit build policy and identified normal/diagnostic artifacts. Include the planned baseline-copy rebuild control for separate-tree comparisons; finish all preparation before collection.
- `benchmark:smoke`: run small correctness/failure checks with explicitly non-acceptance output.
- `benchmark:run`: translate configuration into a core experiment plan for already frozen artifacts; delegate screening/recorded modes, randomized paired controls, placement, lock waiting/progress, collection, cleanup, persistence, and validation to the core.
- `benchmark:summarize`: delegate completion checking and regenerated summaries to core APIs, retaining raw evidence.

Exact task arguments, package metadata schema, compatibility requirements, and feature layout remain open. Registration should reuse ordinary `#[bake::task]`, namespacing, and dependency discovery rather than a new plugin registry.

## Discovery and preparation

Use locked Cargo metadata with explicit package/target opt-in, including `publish=false` workspace members. Permit an explicit list of standalone manifest paths for HTTP/Markdown-style suites. Never recursively execute historical `results/**/Cargo.toml` snapshots. `bake_cargo::packages` deliberately filters for publication and is not a benchmark inventory.

If a standalone CLI also needs Cargo discovery/build preparation, put reusable Bake-independent preparation APIs in `base-benchmark` instead of duplicating them in the adapter. Bake owns task registration, context/configuration translation, and presentation; it does not own a second measurement implementation. Bake JSON/NDJSON output may identify result artifacts but is not the child-sample evidence journal.

Consumer projects will add this provider to their private `bake/Cargo.toml`, regenerate task links, and keep it out of measured/public runtime dependencies. The core must remain usable without Bake. Future integration through `base-project` should follow demonstrated use, not make either published benchmark package depend on `base-project`.

The [scheduler review response](https://github.com/base-rs/base-benchmark/blob/main/reports/external-review-response.md) refines the plan: preparation may target several explicitly prepared trees and a downstream workload, not just this workspace. A core runner executes on the actual measuring host; record developer/collector and measurement host identities separately. A single host-side lock can span preparation followed by collection, with blocking cancellation-aware waiting/status; no builds overlap a measured phase. SSH deployment automation remains undecided. Bake must preserve core distinctions between smoke, non-acceptance screening, and recorded evidence, and between declared performance errors and invalid correctness/completion results.

## Validation and adoption

Historical import/reporting or a comparison UI is a [lower-priority core follow-up](https://github.com/base-rs/base-benchmark/blob/main/context/history.md), not part of the first task family. Any later Bake entry point should delegate to shared validated comparison APIs rather than implement a separate database, web server, or normalization/statistics pipeline.

Use the core's [investigation record convention](https://github.com/base-rs/base-benchmark/blob/main/context/investigations.md) for adopted/rejected/deferred attempts, with raw evidence and rationale retained. A future Bake record helper may delegate to that convention and core metadata; do not create a parallel decision format or treat a rejected candidate as grounds to delete its samples.

Implement the core first; migrate local HTML collection off Python, then test scheduler echo's multiprocess shape. Verify task discovery for unpublished workspace packages and explicit standalone manifests, no execution during listing, build/collection separation, and equivalent validation/results through CLI and Bake. Run normal paths without Python. Shared hosted CI tests correctness and failure behavior, not performance acceptance.
