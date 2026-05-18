# Bamana Task Map

This file tracks explicit development tasks for Bamana native-core milestones.
Milestone 1 is the Native BGZF Core milestone described in
`docs/roadmap/milestone-01-bgzf.md`. Milestone 2 is the Native BAM Header Codec
milestone described in `docs/roadmap/milestone-02-bam-header.md`.

## Milestone 1 Definition

Milestone 1 is complete only when Bamana owns BGZF reading, BGZF writing, EOF
marker handling, and the minimal virtual-offset groundwork needed by later BAM
index and random-access work. The milestone is substrate work first; higher
level command behavior must not hide gaps in BGZF ownership.

## Current Completion State

Status: complete.

Known present pieces:

* native BGZF EOF marker detection exists and is used by `check_eof`;
* shallow BAM verification can inspect a BAM-like BGZF container and first-block
  BAM magic;
* BAM-compatible BGZF writing exists for current first-slice BAM outputs;
* explicit native BGZF `block`, `reader`, `writer`, and `virtual_offset`
  modules are in place;
* unit tests cover BGZF reader, writer, EOF, virtual-offset, and BAM writer
  behaviors;
* `bgzf_microbench` provides runnable native BGZF read, write, EOF, `verify`,
  and `check_eof` timing hooks;
* public contract coverage exists for `benchmark`, `fastq`, and `unmap`;
* `noodles` usage is isolated to the CRAM compatibility module in production
  code.

Milestone 1 closeout evidence:

* all M1.1 through M1.10 tasks are complete;
* final clean-worktree `cargo test` passed with 116 library tests, 14 contract
  tests, binary tests, and doc tests;
* final clean-worktree `cargo test --test contract` passed with 14 contract
  tests;
* final clean-worktree Sphinx build passed for `docs/sphinx`;
* final clean-worktree `bgzf_microbench --profile small --iterations 1
  --bamana-bin target/debug/bamana` ran successfully and reported `ok_count: 1`
  for both `verify` and `check_eof`.

Command-surface scope:

* Milestone 1 evidence is limited to BGZF substrate behavior, `verify` and
  `check_eof` shallow BGZF use, BAM-compatible BGZF writer output, and
  `bgzf_microbench` timings.
* Existing first-slice commands such as `benchmark`, `fastq`, `unmap`,
  `subsample`, `consume`, `sort`, `merge`, and other transform or inspection
  commands may depend on this substrate, but their broader command semantics
  remain downstream work and must not be treated as Milestone 1 closure proof.

## Milestone 2 Definition

Milestone 2 is complete only when Bamana owns BAM header parsing and
deterministic header serialization above the Milestone 1 BGZF substrate. The
milestone covers BAM magic, `l_text`, SAM-style textual header content, the
binary reference dictionary, native error handling for malformed header
prefixes, and the command migration needed for `verify` and `header` to use the
native header path.

## Milestone 2 Current State

Status: active.

Known present pieces:

* Milestone 1 BGZF reading is complete and can inflate the first BAM member;
* `src/bam/header.rs` already contains native first-slice header parsing
  helpers;
* `header` and `verify` exist as public commands and have JSON contracts;
* dependency-boundary checks already prevent production `noodles` usage outside
  the CRAM compatibility exception.

Known gaps:

* Milestone 2 does not yet have completion evidence;
* the current header codec has not been audited against the full M2 acceptance
  criteria;
* malformed-length, reference-dictionary reconciliation, and deterministic
  reserialization behavior need explicit milestone tests;
* header parse and serialization microbenchmarks are not yet present;
* M2.2 through M2.10 remain outstanding.

Milestone 2 closeout evidence must include:

* all M2.1 through M2.10 tasks complete;
* `cargo test` passing;
* `cargo test --test contract` passing;
* Sphinx documentation building successfully;
* header microbenchmarks runnable with machine-readable output;
* roadmap and task-map status updated to record Milestone 2 completion.

Command-surface scope:

* Milestone 2 evidence is limited to native BAM header ownership and the
  `verify` and `header` command paths that consume it.
* `reheader`, `check_sort`, `check_map`, `summary`, `merge`, and checksum
  behavior may benefit from the codec, but their broader semantics remain
  downstream work unless a specific M2 task says otherwise.

## Milestone 1 Task List

### M1.1 Restore Green Verification

Status: complete.

Tasks:

* fix `commands::check_sort::tests::detects_unsorted_coordinate_violation`;
* fix `sampling::hash::tests::fnv1a64_is_stable` by either correcting the
  expected vector or correcting the implementation, with an explicit rationale;
* fix contract-test failures for forensic examples and fixture provenance
  metadata;
* run `cargo test`;
* run `cargo test --test contract`;
* record the final verification commands in the closing commit or PR notes.

Acceptance criteria:

* `cargo test` passes locally;
* `cargo test --test contract` passes locally;
* no failing test is ignored or weakened without a documented reason.

Completion evidence:

* `cargo test` passed with 100 library tests, 11 contract tests, and doc tests;
* `cargo test --test contract` passed with 11 contract tests;
* no test was ignored or weakened.

### M1.2 Make Public Contract Coverage Complete

Status: complete.

Tasks:

* add JSON schemas for `benchmark`, `fastq`, `unmap`, and `annotate_rg`;
* add success and failure examples for `benchmark`, `fastq`, `unmap`, and
  `annotate_rg`;
* update `spec/cli/commands.md` with complete command contracts for
  `benchmark`, `fastq`, and `unmap`;
* update `README.md` and `docs/cli.md` so the documented public CLI matches
  the actual CLI;
* update Sphinx documentation under `docs/sphinx/` for public command behavior
  that users need to operate;
* ensure contract tests enforce the public status of `benchmark`, `fastq`, and
  `unmap`.

Acceptance criteria:

* every public command exposed by `bamana --help` has a schema;
* every public command has at least one success and one failure example;
* contract tests fail if `benchmark`, `fastq`, or `unmap` documentation is
  removed or treated as experimental;
* user-facing and Sphinx docs describe the public commands without implying
  unsupported guarantees.

Completion evidence:

* added schemas and canonical examples for `benchmark`, `fastq`, `unmap`, and
  `annotate_rg`;
* added a contract test that protects `benchmark`, `fastq`, and `unmap` as
  public contract commands;
* updated `README.md`, `docs/cli.md`, `spec/cli/commands.md`, and Sphinx docs;
* `cargo test --test contract` passed with 12 contract tests;
* `cargo test` passed with 100 library tests, 12 contract tests, and doc tests;
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html` passed.

### M1.3 Split BGZF Core Into Explicit Native Modules

Status: complete.

Tasks:

* introduce or complete `src/bgzf/reader.rs`;
* introduce or complete `src/bgzf/writer.rs`;
* introduce or complete `src/bgzf/block.rs`;
* introduce or complete `src/bgzf/virtual_offset.rs`;
* keep `src/bgzf/mod.rs` as the narrow public module facade;
* move shared constants, block metadata, EOF marker handling, and error
  boundaries into the appropriate native BGZF modules;
* update BAM reader and writer consumers to use the clarified BGZF API.

Acceptance criteria:

* BGZF block parsing and block writing are owned by `src/bgzf/*`;
* BAM modules do not duplicate BGZF container logic unnecessarily;
* module names match the roadmap ownership model;
* no production BGZF hot path is implemented through `noodles`.

Completion evidence:

* split the native BGZF substrate into `block`, `reader`, `writer`, and
  `virtual_offset` modules;
* moved `BgzfWriter` ownership from `bam::write` to `bgzf::writer` while
  preserving the existing `bam::write::BgzfWriter` import path as a
  compatibility re-export;
* kept `src/bgzf/mod.rs` as the public facade for EOF, signature, shallow
  first-member, and writer helpers;
* updated the Milestone 1 roadmap module-boundary documentation;
* `cargo test` passed with 100 library tests, 12 contract tests, and doc tests.

### M1.4 Define Virtual Offset Groundwork

Status: complete.

Tasks:

* add a `VirtualOffset` type with compressed-block offset and uncompressed
  in-block offset components;
* validate component bounds according to BGZF virtual-offset semantics;
* provide conversion to and from the packed `u64` representation;
* add ordering tests;
* document which later BAI/CSI and random-access work will consume it.

Acceptance criteria:

* virtual offsets are not passed around as ambiguous raw integers in new BGZF
  code;
* invalid in-block offsets are rejected;
* tests cover packing, unpacking, ordering, and boundary values.

Completion evidence:

* added `bgzf::VirtualOffset` with compressed-block and uncompressed in-block
  components;
* added checked construction with 48-bit compressed-offset and 16-bit
  in-block-offset validation;
* added packed `u64` conversions in both directions;
* added ordering, boundary, packing, unpacking, and invalid-component tests;
* documented BAI/CSI and future random-access consumers in the Milestone 1
  roadmap.

### M1.5 Strengthen BGZF Reader Tests

Status: complete.

Tasks:

* add tests for valid single-block BGZF streams;
* add tests for multi-block streams;
* add tests for truncated block headers;
* add tests for truncated compressed payloads;
* add tests for invalid CRC or size metadata if the reader validates them;
* add tests proving BAM magic can be read from the first inflated BGZF member;
* add tests proving EOF marker presence is detected independently from deeper
  BAM validity.

Acceptance criteria:

* reader tests distinguish container validity, EOF marker presence, first-block
  BAM magic, and full payload validity;
* error responses preserve enough detail for JSON command failures;
* tests are native and do not depend on `noodles` for the production path.

Completion evidence:

* added native reader tests for valid single-block and multi-block BGZF streams;
* added explicit truncated fixed-header, truncated extra-header, and truncated
  compressed-payload tests;
* added invalid CRC/size metadata coverage through the native inflate path;
* added tests proving BAM magic detection is scoped to the first inflated
  member;
* added tests proving BGZF EOF marker detection is independent from BAM magic
  and first-member payload validity;
* tightened reader truncation errors so JSON failures keep BGZF-specific
  details.

### M1.6 Strengthen BGZF Writer Tests

Status: complete.

Tasks:

* add tests for writing an empty BAM-compatible BGZF stream with EOF marker;
* add tests for writing payloads that require multiple BGZF blocks;
* add tests for round-tripping writer output through the native reader;
* add tests for deterministic EOF marker emission;
* add tests for large payload boundaries near the maximum BGZF block payload
  size.

Acceptance criteria:

* native writer output is accepted by the native reader;
* EOF marker behavior is deterministic;
* BAM writer consumers do not need to know BGZF block-layout details.

Completion evidence:

* added native writer tests for empty streams, single-payload round trips,
  multi-block payloads, deterministic EOF marker emission, and payloads at and
  just over the writer target block boundary;
* added a crate-internal native reader helper for reading BGZF payload members
  until the canonical EOF marker;
* verified writer output through the native reader without requiring BAM writer
  consumers to inspect BGZF block layout.

### M1.7 Add BGZF Microbenchmarks

Status: complete.

Tasks:

* define a BGZF read-throughput microbenchmark;
* define a BGZF write-throughput microbenchmark;
* define an EOF-check latency microbenchmark;
* provide small, medium, and large input profiles or clear instructions for
  supplying them;
* document how to run the benchmarks locally;
* ensure benchmark outputs are machine-readable enough to compare future runs.

Acceptance criteria:

* benchmark commands are documented;
* benchmark hooks can run without requiring private data;
* results can be captured in CI or a local benchmark report;
* `verify` and `check_eof` timings can be compared before and after BGZF
  changes.

Completion evidence:

* added `bgzf_microbench`, a standalone Rust microbenchmark binary for native
  BGZF read throughput, write throughput, and EOF-check latency;
* added deterministic `small`, `medium`, and `large` generated input profiles;
* added optional `--bamana-bin` command timings for `verify` and `check_eof`;
* added `benchmarks/results/bgzf_microbench.schema.json` for machine-readable
  result capture;
* documented local benchmark commands in Sphinx, benchmark framework docs, and
  the Milestone 1 roadmap.

### M1.8 Reconcile Roadmap With Implemented Command Surface

Status: complete.

Tasks:

* update roadmap wording so Milestone 1 remains substrate-focused while
  acknowledging that later-wave command slices already exist;
* identify which command behaviors are Milestone 1 evidence and which are
  downstream first slices;
* ensure `docs/roadmap/current_milestone.md`,
  `docs/roadmap/milestone-01-bgzf.md`, and this task map agree;
* keep aspirational items clearly labeled as deferred.

Acceptance criteria:

* readers can tell what is required to close Milestone 1;
* existing later-wave commands are not mistaken for proof that Milestone 1 is
  complete;
* roadmap, task map, and user-facing docs do not contradict each other.

Completion evidence:

* updated the task map completion state to reflect completed BGZF substrate
  work and the then-remaining M1.9/M1.10 closeout path;
* added explicit command-surface scope boundaries to distinguish Milestone 1
  evidence from downstream first-slice command behavior;
* updated the current milestone and Milestone 1 roadmap pages so the milestone
  remains substrate-focused while acknowledging existing later-wave command
  slices;
* labeled deferred work for full BAM header/record ownership, BAI/CSI
  random-access use, broader command migration, and performance optimization.

### M1.9 Confirm Dependency Boundaries

Status: complete.

Tasks:

* audit production `src/` usage of `noodles`;
* keep CRAM compatibility as the only production exception;
* add or update a dependency-boundary test or documented review checklist;
* document any temporary exception with removal criteria.

Acceptance criteria:

* production BAM/BGZF/FASTQ hot paths have no `noodles` dependency;
* CRAM compatibility remains isolated;
* future agents have a clear guardrail for dependency review.

Completion evidence:

* audited production `src/` usage and confirmed direct `noodles_*` references
  are isolated to `src/ingest/cram.rs`;
* added `tests/contract/dependency_boundary.rs` to fail if direct production
  `noodles` usage appears outside the CRAM compatibility boundary;
* updated dependency policy and the noodles demotion plan with the exact CRAM
  exception, guardrail test, and removal criteria.

### M1.10 Close Milestone 1

Status: complete.

Tasks:

* run all required verification commands;
* run BGZF microbenchmarks;
* update Sphinx technical docs;
* update user-facing docs;
* update roadmap status from active/incomplete to complete only after evidence
  is recorded;
* commit and push the closing milestone change.

Acceptance criteria:

* tests pass;
* contract checks pass;
* benchmark hooks are runnable and documented;
* documentation is current;
* Milestone 1 completion evidence is recorded in the repository.

Completion evidence:

* `cargo test` passed in a clean worktree from commit `cea705f` with this
  closeout patch applied, with 116 library tests, 14 contract tests, binary
  tests, and doc tests;
* `cargo test --test contract` passed in the same clean worktree with 14
  contract tests;
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html` passed in the
  same clean worktree;
* `cargo build --bin bamana --bin bgzf_microbench` passed;
* `cargo run --bin bgzf_microbench -- --profile small --iterations 1
  --bamana-bin target/debug/bamana --out /tmp/bamana-m110-bgzf-small.json`
  passed and reported `ok_count: 1` for both `verify` and `check_eof`;
* roadmap and task map status now record Milestone 1 as complete while keeping
  deferred BAM header, BAM record scanning, BAI/CSI random access, and broader
  command migration work outside the milestone.

## Milestone 2 Task List

### M2.1 Activate Milestone 2 Scope And Baseline

Status: complete.

Tasks:

* update `docs/roadmap/current_milestone.md` so Milestone 2 is the active
  milestone and Milestone 1 remains recorded as complete;
* audit existing `src/bam/header.rs`, `src/bam/reader.rs`, and
  `src/bam/write.rs` header behavior against
  `docs/roadmap/milestone-02-bam-header.md`;
* record which existing tests already exercise M2 behavior and which gaps need
  new tests;
* run `cargo test`, `cargo test --test contract`, and the Sphinx build as the
  clean M2 baseline.

Acceptance criteria:

* current milestone documentation names Milestone 2 as active;
* no M1 completion evidence is removed or weakened;
* a baseline verification result is recorded in this task map;
* M2 work starts from a clean or explicitly documented working tree.

Completion evidence:

* updated `docs/roadmap/current_milestone.md` so Milestone 2 is the active
  milestone while preserving Milestone 1 completion evidence;
* audited `src/bam/header.rs`, `src/bam/reader.rs`, `src/bam/write.rs`,
  `src/commands/header.rs`, and `src/commands/verify.rs` against the M2
  roadmap;
* confirmed existing native header pieces: BAM magic parsing, signed `l_text`
  validation, raw SAM-style header preservation, parsed `@HD`, `@SQ`, `@RG`,
  `@PG`, `@CO`, and unknown-record representation, signed `n_ref` validation,
  binary reference parsing, NUL-terminated reference-name validation, signed
  reference length validation, and `serialize_bam_header_payload` for current
  writer consumers;
* confirmed existing tests exercise rich header parsing, binary-reference
  authority when textual `@SQ` length disagrees, writer header round-trip,
  header-only validation mode, and downstream header consumers in sort, unmap,
  merge, subsample, explode, and related commands;
* recorded explicit gaps for M2.2 through M2.10: data-model tightening,
  malformed/truncated header-prefix coverage, text-vs-binary reference
  reconciliation semantics, deterministic parse-serialize-parse coverage,
  `verify` migration from shallow BAM magic to native header validation, and
  header microbenchmarks;
* `cargo test` passed with 120 library tests, 14 contract tests, binary tests,
  and doc tests, with the existing unused-variable warning in
  `src/forensics/forensic_inspect.rs`;
* `cargo test --test contract` passed with 14 contract tests;
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html` passed.

### M2.2 Define Native Header Data Model

Status: complete.

Tasks:

* audit and tighten the native types used for BAM magic, `l_text`, raw header
  text, parsed SAM header records, and binary reference dictionary entries;
* ensure negative lengths and values that do not fit BAM integer fields are
  rejected before allocation or indexing;
* keep unknown SAM-style header records representable without losing their raw
  content;
* document the stable internal representation expected by later `reheader`,
  `merge`, checksum, and validation work.

Acceptance criteria:

* header structures preserve raw header text and parsed reference metadata;
* reference dictionary entries retain name, length, and encounter-order index;
* invalid signed lengths cannot cause large allocations or panics;
* later command consumers do not need to parse BAM header bytes themselves.

Completion evidence:

* documented the native header data model in
  `docs/roadmap/milestone-02-bam-header.md`, including raw SAM-style text
  preservation, binary reference dictionary authority, textual `@SQ` metadata
  retention, unknown-record preservation, and checked serialization through the
  native header module;
* added code-level model documentation to `HeaderPayload`, `BamHeaderView`, and
  `ReferenceRecord`;
* tightened `serialize_bam_header_payload` so writer consumers must pass a
  path and receive `Result<Vec<u8>, AppError>` instead of silently casting
  header text length, reference count, reference-name length, or reference
  length into BAM signed 32-bit fields;
* updated current writer consumers in ingest, transform, forensic, and command
  paths to propagate checked header serialization errors;
* strengthened header tests to assert raw header text preservation, binary
  reference names, lengths, encounter-order indexes, unknown record raw-line
  preservation, and rejection of a reference length that does not fit a BAM
  signed 32-bit field;
* `cargo test` passed with 121 library tests, 14 contract tests, binary tests,
  and doc tests, with the existing unused-variable warning in
  `src/forensics/forensic_inspect.rs`;
* `cargo test --test contract` passed with 14 contract tests;
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html` passed.

### M2.3 Complete Native BAM Header Parsing

Status: pending.

Tasks:

* parse BAM magic from the native BGZF-inflated stream;
* parse and validate `l_text`;
* parse SAM-style textual header records from the declared text span;
* parse and validate `n_ref` and every binary reference dictionary entry;
* detect truncated text, truncated reference names, missing NUL terminators,
  truncated reference lengths, and trailing prefix inconsistencies with clear
  `AppError` details.

Acceptance criteria:

* valid BAM headers parse without relying on `noodles`;
* malformed or truncated header prefixes fail with specific errors;
* tests cover empty headers, headers with references, headers with comments,
  and headers with unknown record types;
* parsing leaves the reader positioned at the first alignment record for later
  scanner work.

Completion evidence:

* pending.

### M2.4 Reconcile Textual And Binary Reference Metadata

Status: pending.

Tasks:

* compare binary reference dictionary entries with textual `@SQ` records;
* define the authoritative source for reference name, length, and order in JSON
  output;
* report mismatches without silently rewriting input semantics;
* preserve duplicate or suspicious textual records as parseable metadata where
  possible;
* add tests for missing `@SQ`, extra `@SQ`, name mismatch, length mismatch, and
  order mismatch cases.

Acceptance criteria:

* binary reference entries remain authoritative for BAM decoding;
* textual `@SQ` metadata is retained and exposed for diagnostics;
* mismatch behavior is deterministic and documented;
* consumers can distinguish strict invalidity from non-fatal header warnings.

Completion evidence:

* pending.

### M2.5 Implement Deterministic Header Serialization

Status: pending.

Tasks:

* serialize native header payloads back to BAM header bytes deterministically;
* preserve or intentionally normalize SAM-style textual header ordering
  according to documented rules;
* serialize the binary reference dictionary in encounter order;
* add round-trip tests for parse-serialize-parse stability;
* expose helper functions that existing BAM writer, checksum, and reheader
  code can share.

Acceptance criteria:

* identical native header values produce byte-identical serialized headers;
* serialized headers are accepted by the native parser;
* reference names include exactly one required NUL terminator in the binary
  dictionary;
* no command duplicates header serialization logic outside the owned module.

Completion evidence:

* pending.

### M2.6 Migrate `header` To The Native Codec

Status: pending.

Tasks:

* route the production `header` command through the native BGZF plus BAM header
  codec path;
* ensure JSON output remains contract-compatible unless a deliberate contract
  update is made in the same task;
* add success and failure examples if native diagnostics alter public output;
* update `README.md`, `docs/cli.md`, `spec/cli/commands.md`, JSON schemas, and
  Sphinx docs if command semantics change.

Acceptance criteria:

* production `header` behavior is not backed by `noodles`;
* existing header command contract tests pass;
* native parse failures are surfaced through structured JSON errors;
* user-facing docs describe the native header scope without implying full BAM
  body validation.

Completion evidence:

* pending.

### M2.7 Migrate `verify` To Native BGZF Plus Native Header

Status: pending.

Tasks:

* replace shallow BAM magic-only verification with native BGZF plus native BAM
  header validation appropriate for M2;
* keep EOF marker reporting scoped to BGZF behavior;
* distinguish BGZF container errors, BAM magic errors, and BAM header prefix
  errors in JSON responses;
* update examples, schemas, CLI docs, and Sphinx docs if `verify` payload or
  failure semantics change.

Acceptance criteria:

* production `verify` uses only Bamana-native BGZF and BAM header code;
* `verify` does not imply full alignment-record validation;
* tests cover valid BAM, invalid BGZF, invalid BAM magic, invalid header text
  length, and truncated reference dictionary cases;
* contract tests continue to protect public `verify` behavior.

Completion evidence:

* pending.

### M2.8 Add Header Oracle And Dependency Boundary Tests

Status: pending.

Tasks:

* add native unit tests for edge-case BAM headers without using `noodles` in the
  production path;
* add oracle or differential tests that may use `noodles` only from tests,
  fixtures, or compatibility helpers;
* extend the dependency-boundary check if new header modules create additional
  risk;
* document the allowed test-only oracle surface and production prohibition.

Acceptance criteria:

* production `header` and `verify` paths remain free of `noodles`;
* test-only oracle usage is clearly isolated;
* malformed-header tests do not depend on external parsers for expected
  failures;
* dependency-boundary verification fails if production header code imports
  `noodles`.

Completion evidence:

* pending.

### M2.9 Add Header Microbenchmarks

Status: pending.

Tasks:

* add a header parse latency microbenchmark;
* add a header serialization microbenchmark;
* add read-prefix or startup-cost timing that can compare `verify` and
  `header` before and after M2 migration;
* emit machine-readable benchmark results;
* document local benchmark commands and result interpretation.

Acceptance criteria:

* benchmarks run without private data;
* benchmark output can be archived under `benchmarks/results/` or an equivalent
  documented path;
* docs explain which timings measure header codec behavior versus full command
  overhead;
* benchmark hooks are included in M2 closeout evidence.

Completion evidence:

* pending.

### M2.10 Close Milestone 2

Status: pending.

Tasks:

* run all required verification commands;
* run header microbenchmarks;
* update technical Sphinx documentation;
* update user-facing docs and CLI contracts affected by the final M2 behavior;
* update `docs/roadmap/milestone-02-bam-header.md`,
  `docs/roadmap/current_milestone.md`, and this task map with final completion
  evidence;
* commit and push the closing milestone change.

Acceptance criteria:

* `cargo test` passes;
* `cargo test --test contract` passes;
* Sphinx documentation builds successfully;
* header microbenchmark hooks are runnable and documented;
* production `verify` and `header` do not rely on `noodles`;
* Milestone 2 completion evidence is recorded in the repository.

Completion evidence:

* pending.
