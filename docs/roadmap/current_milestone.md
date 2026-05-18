# Current Milestone

## Completed Milestone

**Milestone 1: Native BGZF Core**

See:

* [milestone-01-bgzf.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-01-bgzf.md)

## Why This Was Current

Native BGZF ownership is the physical substrate for BAM. It enables:

* EOF checks without external parser dependence
* controlled block reading
* future virtual-offset handling
* the reader and writer foundation used by later BAM milestones

Milestone 1 is substrate-focused. Some downstream command slices already exist
in the repository, but they are not evidence that the native core migration is
complete. They should be read as consumers or early command slices layered on
top of the substrate.

## Completion Status

Milestone 1 is complete. The closing evidence is recorded in:

* [milestone-01-bgzf.md](/Users/stephen/Projects/bamana/docs/roadmap/milestone-01-bgzf.md)
* [../../taskmap.md](/Users/stephen/Projects/bamana/taskmap.md)

Final clean-worktree checks passed:

* `cargo test`
* `cargo test --test contract`
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html`
* `cargo build --bin bamana --bin bgzf_microbench`
* `cargo run --bin bgzf_microbench -- --profile small --iterations 1 --bamana-bin target/debug/bamana`

## What “Done” Means

For contributors, Milestone 1 is done only when:

* BGZF block reading is Bamana-native and exercised by tests
* BGZF EOF behavior is Bamana-native and tested
* BGZF writing is sufficient for BAM-compatible output foundations
* benchmark hooks for read, write, and EOF latency are defined and runnable
* no production BGZF hot path depends on `noodles`

All of those closure criteria are now satisfied for Milestone 1.

## Command-Surface Boundary

Milestone 1 completion evidence:

* `check_eof` uses native BGZF EOF marker handling
* `verify` uses native BGZF first-member inflation for shallow BAM magic checks
* BAM-compatible writer paths can emit native BGZF streams
* `bgzf_microbench` can time native read, write, EOF, `verify`, and `check_eof`

Downstream first slices, including commands such as `benchmark`, `fastq`,
`unmap`, `subsample`, `consume`, `sort`, `merge`, and related inspection or
transform commands, remain outside the Milestone 1 closure criteria except
where they directly exercise the BGZF substrate.

## What Should Not Happen

Do not skip ahead to command-level rewrites that assume a mature native scanner
or header codec before the BGZF substrate is clearly owned.

## Next Milestone

Milestone 2 is the native BAM header codec. It should build on the completed
BGZF substrate rather than reopening Milestone 1 scope.
