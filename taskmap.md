# Bamana Task Map

This file tracks explicit development tasks for Bamana native-core milestones.
Milestone 1 is the Native BGZF Core milestone described in
`docs/roadmap/milestone-01-bgzf.md`. Milestone 2 is the Native BAM Header Codec
milestone described in `docs/roadmap/milestone-02-bam-header.md`. Milestone 3
is the Native BAM Record Scanner milestone described in
`docs/roadmap/milestone-03-bam-record-scan.md`. Milestone 4 is the Native
FASTQ / FASTQ.GZ Parser milestone described in
`docs/roadmap/milestone-04-fastq.md`. Milestone 5 is the Command Migration Off
`noodles` milestone described in
`docs/roadmap/milestone-05-command-migration.md`. Milestone 6 is the Native
Inspection And Validation Commands milestone described in
`docs/roadmap/milestone-06-inspection-validation.md`. Milestone 7 is the
Native Mutation, Remediation, And Forensics Commands milestone described in
`docs/roadmap/milestone-07-mutation-forensics.md`. Milestone 8 is the Native
Transform, Checksum, Explode, And Ingest Commands milestone described in
`docs/roadmap/milestone-08-transform-ingest.md`. Milestone 9 is the Native BAM
Index And Random Access milestone described in
`docs/roadmap/milestone-09-bam-index-random-access.md`. Milestone 10 is the
Native Indexed Region Workflows milestone described in
`docs/roadmap/milestone-10-indexed-region-workflows.md`.

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

Status: complete.

Known present pieces:

* Milestone 1 BGZF reading is complete and can feed native BAM header parsing;
* `src/bam/header.rs` owns native BAM magic, `l_text`, SAM-style header text,
  binary reference dictionary parsing, reconciliation diagnostics, and
  deterministic serialization helpers;
* `header` and `verify` route through native BGZF plus native BAM header paths
  and have governed JSON contracts;
* test-only header oracle coverage is isolated in `tests/header_oracle.rs`;
* `header_microbench` provides runnable native header parse, serialization,
  `verify`, and `header` timing hooks with machine-readable JSON output;
* dependency-boundary checks already prevent production `noodles` usage outside
  the CRAM compatibility exception.

Known gaps:

* none for Milestone 2 scope.

Milestone 2 closeout evidence:

* all M2.1 through M2.10 tasks are complete;
* closeout `cargo test` passed with 151 library tests, 16 contract
  tests, 2 header-oracle integration tests, binary tests, and doc tests, with
  the existing unused-variable warning in `src/forensics/forensic_inspect.rs`;
* closeout `cargo test --test contract` passed with 16 contract
  tests;
* closeout Sphinx build passed for `docs/sphinx`;
* closeout `header_microbench --profile small --iterations 1
  --bamana-bin target/debug/bamana` ran successfully and reported `ok_count: 1`
  for both `verify` and `header`;
* the header microbenchmark JSON smoke check confirmed parse latency,
  serialization latency, and command timings were emitted.

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

Status: complete.

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

* added contextual native BAM reader helpers so header parsing can report which
  header-prefix field was truncated instead of returning only a generic short
  read;
* updated native header parsing to use field-specific reads for `l_text`,
  header text, `n_ref`, reference name length, reference name bytes, and
  reference length;
* tightened reference-name validation to reject empty names and interior NUL
  bytes in addition to missing NUL terminators;
* added tests for empty BAM headers, rich headers with references/comments and
  unknown records, binary-reference preservation, parser positioning at the
  first alignment record, truncated header text, truncated reference count,
  truncated reference name, missing NUL terminator, interior NUL bytes, and
  truncated reference length;
* `cargo test` passed with 129 library tests, 14 contract tests, binary tests,
  and doc tests, with the existing unused-variable warning in
  `src/forensics/forensic_inspect.rs`;
* `cargo test --test contract` passed with 14 contract tests;
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html` passed.

### M2.4 Reconcile Textual And Binary Reference Metadata

Status: complete.

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

* added `header.reference_diagnostics` to the native header model and public
  JSON contract so non-fatal textual `@SQ` reconciliation issues are explicit;
* kept `header.references` binary-authoritative for BAM decoding while
  retaining textual `@SQ` metadata and mismatch details for diagnostics;
* added deterministic diagnostics for missing textual `@SQ` records, extra
  textual `@SQ` records, duplicate textual `@SQ` records, same-index name
  mismatches, order mismatches, length mismatches, and malformed textual `@SQ`
  records without `SN` or parseable `LN`;
* updated the header JSON schema, canonical header success example,
  user-facing CLI docs, JSON-output docs, README, and Milestone 2 roadmap to
  describe reference reconciliation semantics;
* added native tests for missing `@SQ`, extra `@SQ`, name mismatch, length
  mismatch, order mismatch, and duplicate textual `@SQ` cases;
* `cargo test` passed with 136 library tests, 14 contract tests, binary tests,
  and doc tests, with the existing unused-variable warning in
  `src/forensics/forensic_inspect.rs`;
* `cargo test --test contract` passed with 14 contract tests;
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html` passed.

### M2.5 Implement Deterministic Header Serialization

Status: complete.

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

* added `serialize_bam_header_view_payload` so native header views can be
  serialized through the owned BAM header module without command consumers
  assembling BAM header bytes themselves;
* added `serialize_bam_header_checksum_domain` and migrated checksum code away
  from its private header-domain serializer;
* documented deterministic serialization rules in the Milestone 2 roadmap and
  Sphinx technical notes;
* added parse-serialize-parse tests proving native header view serialization is
  byte-stable across repeated calls and accepted by the native parser;
* added tests proving binary reference names are emitted with exactly one NUL
  terminator and binary references remain in encounter order;
* added tests proving checksum-domain header serialization is deterministic;
* `cargo test` passed with 139 library tests, 14 contract tests, binary tests,
  and doc tests, with the existing unused-variable warning in
  `src/forensics/forensic_inspect.rs`;
* `cargo test --test contract` passed with 14 contract tests;
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html` passed.

### M2.6 Migrate `header` To The Native Codec

Status: complete.

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

* added a native BGZF streaming backend that can feed BAM header parsing across
  BGZF member boundaries without using `noodles`;
* routed `src/commands/header.rs` through `parse_bam_header_from_native_bgzf`;
* added command tests proving `header` parses a multi-member native BGZF BAM
  header and surfaces missing BAM magic as `invalid_header`;
* kept the existing JSON response shape and schema unchanged because public
  success and failure semantics did not change;
* updated user-facing and Sphinx documentation to state that `header` validates
  only native BGZF plus BAM header structure, not the BAM body;
* `cargo test` passed with 141 library tests, 14 contract tests, binary tests,
  and doc tests, with the existing unused-variable warning in
  `src/forensics/forensic_inspect.rs`;
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html` passed.

### M2.7 Migrate `verify` To Native BGZF Plus Native Header

Status: complete.

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

* routed `src/commands/verify.rs` through native BGZF recognition and
  `parse_bam_header_from_native_bgzf` instead of stopping after BAM magic;
* kept EOF-marker reporting out of `verify`; `check_eof` remains the BGZF EOF
  command;
* added tests for valid BAM, non-BGZF BAM-shaped input, BGZF without BAM magic,
  negative `l_text`, and a truncated binary reference dictionary;
* updated the `verify` schema, success/failure examples, CLI docs,
  JSON-output docs, Sphinx docs, README, compatibility contract, and Milestone
  2 roadmap to document header-level verification;
* updated the shared `invalid_header` hint so it no longer describes `verify`
  as a shallow pre-header check;
* `cargo test` passed with 146 library tests, 14 contract tests, binary tests,
  and doc tests, with the existing unused-variable warning in
  `src/forensics/forensic_inspect.rs`;
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html` passed.

### M2.8 Add Header Oracle And Dependency Boundary Tests

Status: complete.

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

* added native malformed-header unit tests for invalid UTF-8 header text,
  negative `n_ref`, non-positive reference-name length, non-UTF-8 reference
  names, and negative binary reference lengths;
* added `tests/header_oracle.rs` as the explicit test-only oracle surface using
  `noodles` to compare valid native header parsing and document Bamana's
  binary-authoritative mismatch policy;
* extended dependency-boundary tests so native header, native BGZF reader,
  `header`, and `verify` paths fail if they directly import `noodles`;
* added a contract test requiring the header-oracle policy to document
  `tests/header_oracle.rs`, test-only usage, production prohibition, and native
  malformed-header ownership;
* updated `docs/testing-oracles.md` and Sphinx technical notes with the native
  header oracle boundary;
* `cargo test --test header_oracle` passed with 2 oracle tests;
* `cargo test dependency_boundary --test contract` passed with 4
  dependency-boundary tests;
* `cargo test rejects_ --lib` passed with 21 focused native rejection tests,
  with the existing unused-variable warning in
  `src/forensics/forensic_inspect.rs`;
* `cargo test` passed with 151 library tests, 16 contract tests, 2 header
  oracle integration tests, binary tests, and doc tests, with the existing
  unused-variable warning in `src/forensics/forensic_inspect.rs`;
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html` passed.

### M2.9 Add Header Microbenchmarks

Status: complete.

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

* added `header_microbench`, a standalone Rust microbenchmark binary for native
  BAM header parsing and deterministic header serialization;
* generated deterministic local BAM fixtures for `small`, `medium`, and
  `large` profiles, so benchmark runs require no private input data;
* added optional command startup timings for `verify` and `header` through
  `--bamana-bin`;
* added `benchmarks/results/header_microbench.schema.json` for
  machine-readable benchmark output;
* documented local benchmark commands and interpretation in Sphinx, benchmark
  results docs, performance-core docs, and the Milestone 2 roadmap;
* `cargo build --bin bamana --bin header_microbench` passed;
* `cargo run --bin header_microbench -- --profile small --iterations 1
  --bamana-bin target/debug/bamana --out /tmp/bamana-header-small.json`
  passed;
* JSON smoke validation confirmed `header_parse_latency`,
  `header_serialization_latency`, and `verify`/`header` command timings were
  emitted with successful command exit counts;
* `cargo test` passed with 151 library tests, 16 contract tests, 2 header
  oracle integration tests, binary tests including `header_microbench`, and doc
  tests, with the existing unused-variable warning in
  `src/forensics/forensic_inspect.rs`;
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html` passed.

### M2.10 Close Milestone 2

Status: complete.

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

* ran `cargo test`, which passed with 151 library tests, 16 contract tests, 2
  header-oracle integration tests, binary tests, and doc tests, with the
  existing unused-variable warning in `src/forensics/forensic_inspect.rs`;
* ran `cargo test --test contract`, which passed with 16 contract tests;
* ran `cargo run --bin header_microbench -- --profile small --iterations 1
  --bamana-bin target/debug/bamana --out
  /tmp/bamana-m2-close-header-small.json`, which passed;
* validated the header microbenchmark JSON smoke output, including
  `header_parse_latency`, `header_serialization_latency`, and successful
  `verify`/`header` command timing rows;
* ran `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html`, which
  passed;
* updated `docs/roadmap/milestone-02-bam-header.md`,
  `docs/roadmap/current_milestone.md`, README status text, Sphinx technical
  notes, and this task map with final Milestone 2 completion evidence.

## Milestone 3 Definition

Milestone 3 is complete only when Bamana owns a selective native BAM record
scanner above the Milestone 1 BGZF substrate and Milestone 2 header codec. The
milestone covers record block iteration, bounded structural checks,
lightweight views over core alignment fields, aux-region boundary discovery,
skip-oriented field access, and migration of the first command consumers that
currently need record scans.

## Milestone 3 Current State

Status: complete.

Known present pieces:

* Milestone 1 native BGZF reading is complete and can feed BAM payload bytes;
* Milestone 2 native BAM header parsing is complete and can position readers at
  the first alignment record;
* `src/bam/record.rs` defines `BamRecordView`, a borrowed lightweight record
  view over one complete length-prefixed BAM record;
* `BamRecordView` exposes core fields, sequence length, raw record bytes, and
  stable ranges for the core, read name, CIGAR, sequence, qualities, and aux
  regions without allocating skipped fields;
* `BamRecordView::to_record_layout` provides the explicit bridge back to the
  existing owned `RecordLayout` type for command paths that still need richer
  materialization or lossless serialization;
* `src/bam/scan.rs` defines `BamScanner`, which opens BAM input through the
  native BGZF backend, parses the native BAM header once, and iterates complete
  raw alignment records into `BamRecordView`;
* `BamRecordView` now centralizes selective helpers for flags, coordinates,
  MAPQ, read name, sequence length, section ranges, section presence, borrowed
  section slices, and skip offsets;
* `src/bam/tags.rs` now provides record-view aux helpers for bounded traversal,
  tag lookup, tag counting, tag-key collection, and string tag extraction over
  `BamRecordView::aux_bytes`;
* production `check_sort` now uses `BamScanner` and scanner-owned field helpers
  for record traversal while preserving existing bounded and strict scan
  behavior;
* production `check_map` now preserves index-preferred behavior but routes
  scan fallback record traversal through `BamScanner`;
* production `summary` now routes bounded and full alignment-record scans
  through `BamScanner` and `SummaryAccumulator::observe_view`;
* production `check_tag` now routes aux lookup through `BamScanner` and
  record-view aux helpers;
* production `validate` now uses `BamScanner` and `BamRecordView` for
  record-level structural checks that fit the lightweight view;
* `inspect_duplication` BAM traversal now uses `BamScanner` and borrowed
  record sections for sequence, quality, read-name, and selected RG evidence;
* `forensic_inspect` BAM body scanning now uses `BamScanner` for read-group,
  read-name regime, aux-tag regime, and duplication-hallmark evidence;
* scanner malformed-record coverage is native and includes variable-section
  overflow, unterminated read-name, negative sequence-length, truncated payload,
  negative block-size, and short block-size failures;
* scanner differential coverage compares `BamRecordView` against Bamana's owned
  `RecordLayout` bridge on valid records without using an external parser;
* dependency-boundary tests now explicitly protect the scanner substrate and
  migrated record hot paths from direct `noodles` imports;
* `scanner_microbench` provides runnable records-per-second and selective field
  extraction benchmark hooks with machine-readable JSON output;
* `src/bam/records.rs` contains the current central record bridge through
  `read_next_record_layout`, which performs bounded layout checks and
  materializes read name, CIGAR, sequence, quality, and aux sections;
* `LightAlignmentRecord` already exposes many field-only command inputs, but it
  is derived from `RecordLayout` and is not yet a true selective scanner view;
* `src/bam/tags.rs` contains bounded aux traversal and tag lookup over
  materialized aux bytes;
* BAM-side `subsample` and writer-heavy transform paths still perform
  record-facing work through existing first-slice readers and helpers, which
  is downstream command-migration work rather than Milestone 3 scanner
  substrate work;
* dependency-boundary checks already prohibit production `noodles` usage
  outside the CRAM compatibility exception.

Known gaps:

* remaining BAM-side transform consumers are not migrated onto a shared scanner
  substrate where command behavior needs owned records or writer-heavy
  serialization;
* richer decode and lossless serialization paths still use `RecordLayout` where
  command behavior needs owned sequence, quality, aux, or whole-record bytes.

Milestone 3 closeout evidence:

* all M3.1 through M3.10 tasks are complete;
* closeout `cargo test` passed with 180 library tests, 18 contract tests, 2
  header-oracle integration tests, binary tests, and doc tests;
* closeout `cargo test --test contract` passed with 18 contract tests;
* closeout Sphinx build passed for `docs/sphinx`;
* closeout `cargo run --bin scanner_microbench -- --profile small --iterations
  1` passed, and the JSON smoke check verified the small profile, one
  iteration, 1,024 generated records, and the expected result schema;
* selected first command consumers documented as using the native scanner are
  `check_sort`, `check_map`, `summary`, `check_tag`, `validate`,
  `inspect_duplication`, and `forensic_inspect`;
* roadmap and task-map status now record Milestone 3 completion.

Command-surface scope:

* Milestone 3 evidence is limited to native BAM record scanning and the
  selected command paths that consume scanner-owned record views.
* Full semantic BAM validation, BAI/CSI random access, native CRAM scanning,
  broad command parity, and expensive full generic record decoding are
  downstream unless a specific M3 task explicitly includes them.

## Milestone 3 Task List

### M3.1 Activate Milestone 3 Scope And Baseline

Status: complete.

Tasks:

* update `docs/roadmap/current_milestone.md` so Milestone 3 is the active
  milestone and Milestones 1 and 2 remain recorded as complete;
* update user-facing and Sphinx documentation to state the active scanner
  scope without implying full BAM validation or command-wide migration;
* audit `src/bam/records.rs`, `src/bam/reader.rs`, and first record-scanning
  command consumers against `docs/roadmap/milestone-03-bam-record-scan.md`;
* record the existing helpers, first consumer order, and explicit scanner gaps
  before scanner code is introduced.

Acceptance criteria:

* current milestone documentation names Milestone 3 as active;
* the task map records baseline present pieces and gaps for Milestone 3;
* Sphinx and user-facing docs describe the scanner milestone boundary;
* no runtime behavior changes are made unless required by the documentation
  audit.

Completion evidence:

* updated `docs/roadmap/current_milestone.md` so Milestone 3 is active and
  Milestones 1 and 2 remain recorded as complete;
* updated README and Sphinx documentation to describe the active scanner scope
  and boundary without claiming full BAM validation or command-wide migration;
* audited `src/bam/records.rs` and recorded that `read_next_record_layout`
  currently validates core layout and materializes read name, CIGAR, sequence,
  quality, and aux bytes into `RecordLayout`;
* recorded that `read_next_light_record` derives `LightAlignmentRecord` from
  the full layout path, so current field-only consumers still pay for full
  materialization;
* audited `src/bam/reader.rs` and recorded that record consumers generally use
  the transitional `BamReader::open` gzip backend while a native BGZF backend
  is already available for the scanner substrate;
* audited first record-scanning consumers and recorded the migration order:
  `check_sort`; then `check_map`, `summary`, and `check_tag`; then
  `validate`, `inspect_duplication`, and `forensic_inspect`; then BAM-side
  `subsample` and other raw-record writers after scanner raw-record access or
  lossless `RecordLayout` bridging is available;
* updated `docs/roadmap/milestone-03-bam-record-scan.md` with the baseline
  audit, first consumer order, and explicit gaps before scanner code is
  introduced;
* no runtime behavior changes were made.

### M3.2 Define Lightweight Record View Contract

Status: complete.

Tasks:

* define the native lightweight BAM record view API for core fields including
  `refID`, `pos`, flags, MAPQ, read name, sequence length, and aux-region
  boundaries;
* expose raw record bounds and range metadata needed to skip CIGAR, sequence,
  qualities, and aux sections without allocating skipped data;
* decide which existing richer record conversion helpers remain as bridge
  APIs for command paths that still need full materialization;
* add focused tests for valid minimal records, variable-length record layout,
  and boundary arithmetic.

Acceptance criteria:

* required M3 fields are inspectable without full generic decode;
* malformed or truncated variable regions cannot produce out-of-bounds slices;
* skipped fields do not allocate merely to advance through a record;
* richer conversion remains possible where downstream command behavior still
  needs it.

Completion evidence:

* added `src/bam/record.rs` with `BamRecordView<'a>` and
  `BamRecordSections`;
* `BamRecordView::parse` validates a complete length-prefixed BAM record and
  exposes `refID`, `pos`, flags, MAPQ, read name, sequence length, mate/core
  fields, raw record bytes, and stable ranges for core, read name, CIGAR,
  sequence, qualities, and aux bytes;
* section accessors return borrowed slices for CIGAR, sequence, qualities, and
  aux regions so field-only consumers can skip data without allocation once the
  scanner feeds this view;
* added `BamRecordView::to_record_layout` as the explicit bridge for existing
  richer consumers that still need owned `RecordLayout` materialization or
  lossless serialization;
* added tests for a valid minimal record, variable-length section boundary
  arithmetic, lossless bridge serialization, oversized variable-section
  rejection, and non-NUL-terminated read-name rejection;
* updated Sphinx and roadmap documentation to describe the record-view
  contract;
* `cargo test bam::record` passed.

### M3.3 Implement Native Scan Loop

Status: complete.

Tasks:

* introduce the shared native scanner module, expected to be `src/bam/scan.rs`;
* iterate BAM records from a native BGZF reader after native header parsing;
* handle clean EOF, truncated block sizes, negative or impossible record sizes,
  and malformed record prefixes with structured errors;
* preserve input path context in scanner-facing errors where command surfaces
  need it;
* add tests for empty BAM bodies, single-record bodies, multi-record bodies,
  and truncated records.

Acceptance criteria:

* scanner iteration reads records in encounter order without a generic
  external decoder;
* valid records produce lightweight views;
* malformed records fail deterministically with specific errors;
* no production scanner hot path imports `noodles`.

Completion evidence:

* added `src/bam/scan.rs` with `BamScanner`;
* `BamScanner::open` uses `BamReader::open_native_bgzf` and
  `parse_bam_header_from_reader` so scanning starts after the native BGZF and
  native BAM header path;
* `BamScanner::next_record` reads the length-prefixed raw record payload,
  preserves path context, and returns `BamRecordView` for valid records;
* clean EOF returns `Ok(None)` without treating the canonical BGZF EOF marker
  as an alignment record;
* negative record sizes, block sizes smaller than the BAM core, truncated block
  sizes, truncated payloads, and `BamRecordView` parse failures surface as
  structured `AppError` values;
* added scanner tests for empty BAM bodies, single-record bodies, multi-record
  encounter order, negative block sizes, block sizes smaller than the core,
  truncated payloads, and truncated block-size prefixes;
* no production scanner hot path imports `noodles`;
* `cargo test bam::scan` passed.

### M3.4 Add Selective Field Extraction Helpers

Status: complete.

Tasks:

* centralize helpers for flags, coordinates, MAPQ, read name, sequence length,
  and aux bounds;
* add skip helpers for CIGAR, sequence, qualities, and aux payload when a
  consumer does not need those fields;
* cover records with and without CIGAR data, aux data, sequence bases, and
  qualities;
* document which helpers are stable scanner APIs versus internal layout
  helpers.

Acceptance criteria:

* common scan consumers do not duplicate byte-offset arithmetic;
* skipped field access remains bounded and allocation-light;
* tests cover representative mapped, unmapped, and minimal record layouts.

Completion evidence:

* added `BamRecordFlags`, `BamRecordCoordinates`, and
  `BamRecordSkipOffsets` as scanner-facing helper value types;
* added `BamRecordView::flag_summary`, `coordinates`, section range helpers,
  `skip_offsets`, `has_cigar`, `has_sequence`, `has_qualities`, and `has_aux`;
* kept CIGAR, sequence, quality, and aux data access as borrowed slices so
  consumers can inspect selected sections without owned allocation;
* documented the stable scanner helper surface in roadmap and Sphinx docs,
  while keeping parser internals and the `RecordLayout` bridge separate;
* added tests for populated variable sections, mapped flag/coordinate helpers,
  unmapped flag helpers, minimal records, empty sections, and skip offsets;
* `cargo test bam::record` passed.

### M3.5 Add Aux Region Traversal Without Full Decode

Status: complete.

Tasks:

* implement bounded aux tag iteration over the aux region exposed by the record
  view;
* support tag lookup and type skipping for `check_tag`, read-group evidence,
  forensic scans, and checksum inputs that only need selected tags;
* reject malformed aux payloads with structured errors instead of silently
  truncating or over-reading;
* add tests for scalar tags, string tags, array tags, missing tags, duplicate
  tags, and malformed aux lengths.

Acceptance criteria:

* selected aux tags can be found without decoding every optional field into a
  rich record object;
* unsupported or malformed aux shapes are reported precisely;
* scanner consumers can distinguish absent tags from malformed aux data.

Completion evidence:

* added scanner-facing record-view aux helpers in `src/bam/tags.rs`:
  `traverse_record_aux_fields`, `record_aux_contains_tag`,
  `count_record_aux_tag`, `collect_record_aux_tag_keys`, and
  `extract_record_string_aux_tag`;
* reused the existing bounded aux payload parser, so scanner consumers get one
  shared type-skipping path for scalar tags, strings, hex strings, and
  B-arrays instead of a second decoder;
* helpers operate on `BamRecordView::aux_bytes`, so selected tag lookup and
  read-group evidence do not require `RecordLayout` materialization;
* malformed aux payloads still report precise parse errors for truncated
  scalar fields, unterminated strings, truncated arrays, negative array counts,
  unsupported B-array subtypes, and unsupported aux type codes;
* added tests for record-view scalar tag lookup, string RG extraction,
  B-array skipping before later tags, missing-tag absence, duplicate-tag
  counting, malformed aux lengths, and borrowed aux traversal;
* `cargo test bam::tags` passed.

### M3.6 Migrate `check_sort` To The Native Scanner

Status: complete.

Tasks:

* route `check_sort` record iteration through the native scanner;
* preserve current JSON success, warning, and error contracts unless deliberate
  contract updates are made in the same task;
* keep bounded sampling behavior and strict-mode behavior intact;
* update command tests, examples, schemas, README, CLI docs, and Sphinx docs if
  public semantics change.

Acceptance criteria:

* production `check_sort` scan behavior uses the native scanner;
* no command-local duplicated record scan loop remains for scanner-owned
  fields;
* existing contract tests pass;
* user-facing docs do not imply full BAM validation.

Completion evidence:

* routed production `check_sort` through `BamScanner::open` and
  `BamScanner::next_record`;
* removed `check_sort` usage of `BamReader::open`,
  `parse_bam_header_from_reader`, and `read_next_light_record`;
* built the command's sort-only comparison snapshot from `BamRecordView` plus
  scanner-owned flag helpers instead of full `RecordLayout` materialization;
* preserved existing bounded scan, strict scan, specialized-sort,
  coordinate-sort, queryname-sort, JSON payload, and semantic-note behavior;
* no command contract update was required because public output semantics did
  not change;
* `cargo test commands::check_sort` passed.

### M3.7 Migrate `check_map`, `summary`, And `check_tag` Scanner Consumers

Status: complete.

Tasks:

* route scan-based `check_map` fallback behavior through the native scanner
  while preserving index-preferred behavior where applicable;
* route `summary` bounded and full record scans through scanner-owned field
  extraction;
* route `check_tag` aux traversal through scanner-owned aux helpers;
* update contracts and documentation if any command payload or diagnostic
  changes.

Acceptance criteria:

* these command consumers share scanner primitives for record traversal;
* index-derived behavior remains preferred where already documented;
* JSON contracts remain stable or are deliberately versioned;
* tests cover representative scanner-backed command paths.

Completion evidence:

* routed scan-based `check_map` fallback behavior through `BamScanner` while
  leaving usable BAI index summaries preferred;
* routed production `summary` bounded and full scans through `BamScanner` and
  scanner-owned field extraction via `SummaryAccumulator::observe_view`;
* routed production `check_tag` aux traversal through `BamScanner` and
  `record_aux_contains_tag`;
* preserved the existing JSON payload contracts and diagnostics;
* added command-level scanner-backed tests for `summary` and `check_tag`;
* `cargo test commands::check_map`, `cargo test commands::summary`, and
  `cargo test commands::check_tag` passed.

### M3.8 Migrate Validation And Forensics First-Slice Consumers

Status: complete.

Tasks:

* migrate `validate` record-access checks that fit the lightweight scanner
  view;
* migrate `inspect_duplication` BAM record traversal that only needs scanner
  fields or selected aux evidence;
* migrate `forensic_inspect` BAM record-facing helpers that can consume
  scanner-owned views;
* explicitly document any remaining richer decode paths that are deferred
  beyond Milestone 3.

Acceptance criteria:

* validation and forensic first-slice paths use the scanner for common
  structural traversal where practical;
* deferred rich decode gaps are named rather than hidden;
* public documentation preserves command caveats and avoids overclaiming full
  semantic validation.

Completion evidence:

* migrated `validate_bam` record traversal to `BamScanner::open` and
  `BamScanner::next_record`;
* migrated scanner-compatible validation checks to `BamRecordView` fields and
  `traverse_record_aux_fields`;
* migrated `inspect_duplication` BAM traversal to `BamScanner`, borrowed
  sequence/quality slices, and `extract_record_string_aux_tag`;
* migrated `forensic_inspect` BAM body scanning to `BamScanner`, borrowed
  read-name/sequence/quality slices, and record-view aux helpers;
* documented that remaining writer-heavy transform work and richer owned
  decode/serialization paths remain deferred;
* `cargo test bam::validate`, `cargo test forensics::duplication`, and
  `cargo test forensics::forensic_inspect` passed.

### M3.9 Add Scanner Oracle, Dependency Boundary, And Microbenchmarks

Status: complete.

Tasks:

* add native malformed-record tests that do not depend on external parsers for
  expected failures;
* add any useful test-only oracle or differential tests while keeping oracle
  usage outside production code;
* extend dependency-boundary tests so production scanner and migrated command
  hot paths cannot import `noodles`;
* add records-per-second and selective-field-extraction microbenchmark hooks
  with machine-readable output;
* document benchmark commands and output interpretation.

Acceptance criteria:

* scanner malformed-record behavior is covered by native tests;
* production scanner and migrated hot paths remain free of direct `noodles`
  usage;
* scanner benchmarks are runnable without private data;
* benchmark JSON can be archived or validated by a documented schema.

Completion evidence:

* added native malformed scanner tests for variable-section overflow,
  unterminated read names, negative sequence lengths, truncated payloads,
  truncated block sizes, negative block sizes, and short block sizes;
* added a native scanner differential test comparing `BamRecordView` to the
  owned `RecordLayout` bridge for a valid record;
* extended dependency-boundary contract tests to protect scanner and migrated
  record hot paths from direct `noodles` imports;
* documented the scanner oracle boundary in `docs/testing-oracles.md`;
* added `scanner_microbench` with records-per-second and selective
  field-extraction measurements;
* added `benchmarks/results/scanner_microbench.schema.json` and Sphinx
  benchmark documentation;
* `cargo build --bin scanner_microbench` passed;
* `cargo test bam::scan` passed;
* `cargo test dependency_boundary --test contract` passed;
* `cargo run --bin scanner_microbench -- --profile small --iterations 1`
  passed and emitted benchmark JSON.

### M3.10 Close Milestone 3

Status: complete.

Tasks:

* run `cargo test`;
* run `cargo test --test contract`;
* run the scanner microbenchmark smoke profile;
* run the Sphinx documentation build;
* update `docs/roadmap/milestone-03-bam-record-scan.md`,
  `docs/roadmap/current_milestone.md`, README status text, Sphinx technical
  notes, and this task map with final Milestone 3 completion evidence;
* commit and push the closing milestone change.

Acceptance criteria:

* all M3.1 through M3.10 tasks are complete;
* full tests, contract tests, Sphinx, and scanner microbenchmarks pass;
* command migration evidence is recorded for selected scanner consumers;
* production BAM record hot-path scanning does not depend on `noodles`;
* Milestone 3 completion evidence is recorded in the repository.

Completion evidence:

* `cargo test` passed with 180 library tests, 18 contract tests, 2
  header-oracle integration tests, binary tests, and doc tests;
* `cargo test --test contract` passed with 18 contract tests;
* `cargo run --bin scanner_microbench -- --profile small --iterations 1`
  passed, and a JSON smoke check verified the benchmark profile, iteration
  count, 1,024 generated records, and expected result schema;
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html` passed;
* updated `docs/roadmap/milestone-03-bam-record-scan.md`,
  `docs/roadmap/current_milestone.md`, README status text, Sphinx technical
  notes, and this task map with final Milestone 3 completion evidence;
* selected scanner consumers are documented as `check_sort`, `check_map`,
  `summary`, `check_tag`, `validate`, `inspect_duplication`, and
  `forensic_inspect`.

## Milestone 4 Definition

Milestone 4 is complete only when Bamana owns a native FASTQ and FASTQ.GZ
parser/writer core that is explicit enough for ingest, subsampling,
duplication inspection, deduplication, indexing, enumeration, and benchmark
work to depend on it. The milestone covers plain FASTQ record parsing,
FASTQ.GZ stream handling, record validation, allocation-conscious record
views, valid FASTQ writing, gzip writing, existing `FASTQ.GZI` sidecar
integration, command consumer migration, and benchmark hooks.

## Milestone 4 Current State

Status: complete.

Known present pieces:

* `src/fastq/mod.rs` is the Bamana-native FASTQ facade and preserves
  compatibility imports for existing command consumers;
* `src/fastq/record.rs` owns `FastqRecord`, `FastqRecordView`, read-name
  parsing, record-line validation, plus-line preservation, field accessors,
  and FASTQ identity-byte construction;
* `src/fastq/reader.rs` owns plain/gzip reader opening, record parsing, record
  validation, and record counting;
* `src/fastq/writer.rs` owns plain/gzip FASTQ writing and finish behavior;
* `src/fastq/gzip.rs` owns extension-based gzip detection, `MultiGzDecoder`
  reader construction, and shared FASTQ thread-count resolution;
* `src/fastq/unmapped.rs` owns unmapped-BAM conversion,
  threaded FASTQ.GZ-to-BAM conversion, and selected HTS-style methylation
  header tag conversion;
* `src/fastq/gzi.rs` owns the current `FASTQ.GZI` sidecar builder, reader,
  checkpoint sampler, and explode range planner;
* `src/ingest/fastq.rs` is now a compatibility shim that re-exports
  `crate::fastq`;
* `read_next_fastq_record` validates the four-line FASTQ structure, header
  marker, plus marker, sequence/quality length equality, and usable read name;
* `open_fastq_reader_with_label` reads plain FASTQ and `.gz` inputs with
  `flate2::read::MultiGzDecoder`;
* `FastqWriter` writes plain FASTQ or gzip-compressed FASTQ according to the
  output extension;
* `enumerate` uses `count_fastq_records` for plain FASTQ and
  `ensure_fastq_gzi` for FASTQ.GZ counting and sidecar reuse;
* FASTQ-side `subsample` uses `FastqRecord`, `FastqWriter`,
  `open_fastq_reader`, and `read_next_fastq_record`;
* `consume` uses the native FASTQ helpers for raw-read to unmapped-BAM
  normalization, including indexed single-FASTQ.GZ and multi-FASTQ.GZ worker
  paths;
* `inspect_duplication` and `deduplicate` already consume the native FASTQ
  helpers for FASTQ and FASTQ.GZ inputs;
* `explode` uses FASTQ helpers plus `FASTQ.GZI` planning for FASTQ.GZ shards;
* public contract coverage already treats `benchmark`, `fastq`, and `unmap`
  as governed public commands.

Known gaps:

* the current streaming reader still returns owned `FastqRecord` values, so
  field-only streaming integration remains future work even though
  `FastqRecordView` exists for borrowed access when line storage is already
  available;
* gzip behavior is extension-driven and uses `MultiGzDecoder` with documented
  single-member and concatenated multi-member FASTQ.GZ semantics;
* writer behavior preserves owned record content, uses LF line endings,
  selects gzip output from the final `.gz` extension, and finalizes or flushes
  output before returning success;
* selected command consumers are audited against the stable Milestone 4
  reader/writer API, while richer command-specific FASTQ behavior remains
  deferred to later command-migration work;
* FASTQ microbenchmark hooks are present through `fastq_microbench`, and M4.10
  records closeout smoke-run evidence.

Milestone 4 closeout evidence must include:

* all M4.1 through M4.10 tasks complete;
* `cargo test` passing;
* `cargo test --test contract` passing;
* Sphinx documentation building successfully;
* FASTQ parser/writer benchmark or smoke benchmark runnable with
  machine-readable output;
* command migration evidence recorded for FASTQ-side `subsample`, `consume`,
  `inspect_duplication`, `deduplicate`, `enumerate`, and relevant
  `FASTQ.GZI`/`explode` consumers;
* dependency-boundary evidence showing FASTQ hot paths remain Bamana-native
  and do not depend on external generic bioinformatics parser crates.

Command-surface scope:

* Milestone 4 evidence is limited to native FASTQ and FASTQ.GZ parsing,
  writing, record validation, sidecar-aware enumeration/planning, and selected
  command consumers that use those primitives.
* Paired-read reconciliation, biological quality interpretation, adapter
  trimming, richer ONT/PacBio metadata semantics, and full comparator parity
  remain downstream unless a specific M4 task explicitly includes them.

## Milestone 4 Task List

### M4.1 Activate Milestone 4 Scope And Baseline

Status: complete.

Tasks:

* update `docs/roadmap/current_milestone.md` so Milestone 4 is the active
  milestone and Milestones 1 through 3 remain recorded as complete;
* update roadmap/task-map status so the top-level milestone index no longer
  names Milestone 3 as active;
* audit `src/fastq/mod.rs`, `src/fastq/gzi.rs`, `src/ingest/fastq.rs`, and
  FASTQ-facing command consumers;
* record the existing parser, writer, gzip, sidecar, and command-consumer
  baseline before refactoring modules;
* update user-facing and Sphinx documentation if the active milestone status
  changes visible project guidance.

Acceptance criteria:

* current milestone documentation names Milestone 4 as active;
* task-map baseline lists present FASTQ functionality and known gaps;
* no runtime behavior changes are made unless required by the audit;
* public contract commands remain explicitly protected.

Completion evidence:

* `docs/roadmap/current_milestone.md` names Milestone 4 as active as of
  2026-05-18 and records Milestones 1 through 3 as complete;
* `docs/roadmap.md` lists Milestone 4 as active and Milestones 1 through 3 as
  complete, so the top-level milestone index no longer names Milestone 3 as
  active;
* audited `src/fastq/mod.rs` and confirmed the current native FASTQ surface
  includes `FastqRecord`, reader opening for plain and `.gz` inputs,
  `read_next_fastq_record`, record counting, unmapped-BAM conversion,
  threaded FASTQ.GZ-to-BAM conversion, `FastqWriter`, and plain/gzip writing;
* audited `src/fastq/gzi.rs` and confirmed it owns `FASTQ.GZI` build, read,
  checkpoint sampling, metadata flags, and explode range planning;
* audited `src/ingest/fastq.rs` and confirmed it is only a compatibility shim
  re-exporting `crate::fastq`;
* audited FASTQ-facing consumers and confirmed `enumerate`, FASTQ-side
  `subsample`, `consume`, `inspect_duplication`, `deduplicate`, `index`, and
  FASTQ.GZ `explode` already call native FASTQ helpers in some form;
* recorded baseline gaps in the Milestone 4 current-state section: monolithic
  `src/fastq/mod.rs`, owned-string `FastqRecord`, extension-driven gzip
  behavior, incomplete gzip stream semantics tests, writer contract gaps,
  pending command-consumer API audit, and missing FASTQ benchmark closeout
  evidence;
* no runtime behavior changes were made for M4.1;
* public contract commands remain explicitly protected by existing contract
  coverage for `benchmark`, `fastq`, and `unmap`.

### M4.2 Split FASTQ Core Into Explicit Native Modules

Status: complete.

Tasks:

* introduce or complete `src/fastq/record.rs`;
* introduce or complete `src/fastq/reader.rs`;
* introduce or complete `src/fastq/writer.rs`;
* introduce or complete an explicit gzip/FASTQ.GZ boundary module if the
  existing reader/writer split needs it;
* keep `src/fastq/mod.rs` as the narrow public facade;
* preserve compatibility imports used by existing command consumers during the
  split.

Acceptance criteria:

* FASTQ record types, reader logic, writer logic, gzip handling, and `FASTQ.GZI`
  sidecar logic have clear module ownership;
* existing command behavior is preserved;
* no FASTQ hot path is moved into `src/ingest/fastq.rs`;
* focused module tests pass.

Completion evidence:

* added `src/fastq/record.rs` for `FastqRecord` and read-name parsing;
* added `src/fastq/reader.rs` for reader opening, record parsing, validation,
  and record counting;
* added `src/fastq/writer.rs` for `FastqWriter`, `write_fastq_records`, and
  plain/gzip finish behavior;
* added `src/fastq/gzip.rs` for extension-driven gzip detection,
  `MultiGzDecoder` reader construction, and shared FASTQ thread-count
  resolution;
* added `src/fastq/unmapped.rs` for FASTQ-to-unmapped-BAM conversion,
  threaded FASTQ.GZ conversion, read-group aux emission, and selected
  HTS-style methylation header tag conversion;
* kept `src/fastq/mod.rs` as a narrow public facade that re-exports the stable
  functions and types used by existing command consumers;
* left `src/fastq/gzi.rs` as the owner of `FASTQ.GZI` sidecar logic;
* kept `src/ingest/fastq.rs` as a compatibility shim only;
* added Sphinx technical documentation for the native FASTQ core module
  ownership in `docs/sphinx/native_fastq_core.rst`;
* focused module tests passed with `cargo test fastq:: --lib`.

### M4.3 Define Native FASTQ Record View And Owned Record Contract

Status: complete.

Tasks:

* define the stable owned FASTQ record type for consumers that need retained
  records;
* define a borrowed or allocation-conscious record view for field-only
  consumers where practical;
* centralize read-name parsing, raw header retention, plus-line handling,
  sequence access, quality access, and identity-byte helpers;
* document which APIs are stable scanner/parser contracts versus internal
  layout helpers.

Acceptance criteria:

* consumers do not duplicate FASTQ read-name or identity parsing;
* field-only readers can avoid unnecessary copies where the reader design
  permits it;
* owned record behavior remains available for `subsample`, `deduplicate`, and
  writer-heavy paths;
* tests cover headers with comments, empty comments, plus-line comments, and
  representative sequence/quality payloads.

Completion evidence:

* kept `FastqRecord` as the stable owned FASTQ record for consumers that retain
  records or write them back out;
* added `FastqRecordView` as a borrowed view over raw header, parsed read name,
  sequence, plus line, and quality fields;
* moved record-line validation into `FastqRecord::from_lines` and
  `FastqRecordView::from_lines`, preserving existing error detail text for the
  reader path;
* centralized read-name parsing in `src/fastq/record.rs` so the reader and
  unmapped-BAM conversion path share the same parser;
* added `FastqIdentityBasis` and `identity_bytes` helpers for qname,
  qname-sequence, and full-record FASTQ deterministic identities;
* routed deterministic FASTQ `subsample` identity hashing through
  `FastqRecord::identity_bytes` instead of a command-local duplicate helper;
* routed `FastqWriter` through `FastqRecordView::lines` so plus-line handling
  and line order stay owned by the FASTQ record contract;
* documented the owned/view record contract in
  `docs/sphinx/native_fastq_core.rst`;
* focused tests passed with `cargo test fastq:: --lib` and
  `cargo test commands::subsample::tests::deterministic_fastq_subsampling_is_repeatable --lib`.

### M4.4 Strengthen Plain FASTQ Reader Validation

Status: complete.

Tasks:

* add reader tests for valid single-record and multi-record plain FASTQ;
* add tests for missing sequence, plus, and quality lines;
* add tests for invalid header and plus markers;
* add tests for sequence/quality length mismatches;
* add tests for blank read names and edge-case line endings;
* ensure errors include input path context and precise failure details.

Acceptance criteria:

* malformed plain FASTQ fails deterministically with structured
  `InvalidFastq` errors;
* valid records preserve encounter order and raw line semantics required by
  existing writers;
* tests cover both EOF and truncated-record boundaries.

Completion evidence:

* added `src/fastq/reader.rs` tests for valid single-record parsing followed
  by clean EOF;
* added `src/fastq/reader.rs` tests for valid multi-record plain FASTQ parsing
  in encounter order;
* added reader tests proving raw header comments, plus-line comments, sequence
  text, and quality text are preserved after parsing;
* added CRLF line-ending coverage proving trailing `\r\n` is normalized without
  corrupting FASTQ line semantics;
* added clean empty-input EOF coverage;
* added truncated-record tests for missing sequence, missing plus, and missing
  quality boundaries;
* added invalid-marker tests for malformed header and plus lines;
* added malformed-content tests for sequence/quality length mismatch and blank
  read name;
* malformed reader tests assert structured `AppError::InvalidFastq` errors
  include the input path and precise detail text;
* focused tests passed with `cargo test fastq::reader::tests --lib` and
  `cargo test fastq:: --lib`.

### M4.5 Strengthen FASTQ.GZ Reader And Gzip Stream Semantics

Status: complete.

Tasks:

* add tests for valid single-member FASTQ.GZ;
* add tests for valid multi-member FASTQ.GZ;
* add tests for truncated or corrupt gzip streams;
* document the supported gzip semantics for FASTQ.GZ inputs;
* make extension-based gzip detection explicit and auditable, or replace it
  with a better local policy if needed;
* preserve compatibility with `FASTQ.GZI` sidecar creation and reuse.

Acceptance criteria:

* FASTQ.GZ parsing supports the documented gzip member behavior;
* corrupt or truncated gzip input reports structured errors instead of silent
  count or record truncation;
* `enumerate`, `consume`, and `explode` sidecar paths still pass.

Completion evidence:

* added `src/fastq/gzip.rs` tests proving extension-based gzip detection is
  explicit, case-insensitive for `.gz`, and does not treat non-final `.gz`
  components as gzip inputs;
* added a valid single-member FASTQ.GZ parser/counting test through the public
  FASTQ reader facade;
* added a valid concatenated multi-member FASTQ.GZ parser/counting test proving
  records are yielded in gzip member order;
* added corrupt gzip and truncated gzip tests proving failures return
  structured `AppError::Io` values with input path context instead of silent
  count or record truncation;
* added a `FASTQ.GZI` build test for a concatenated multi-member FASTQ.GZ
  stream;
* documented supported FASTQ.GZ semantics in
  `docs/sphinx/native_fastq_core.rst`: extension-selected gzip decoding,
  `MultiGzDecoder`, concatenated member support, structured corrupt/truncated
  stream errors, and `FASTQ.GZI` compatibility;
* focused gzip tests passed with `cargo test fastq::gzip::tests --lib`;
* focused FASTQ tests passed with `cargo test fastq:: --lib`;
* sidecar consumer smoke tests passed for FASTQ.GZ `enumerate`, FASTQ.GZ
  `explode`, and indexed FASTQ.GZ `consume`.

### M4.6 Complete FASTQ Writer And Round-Trip Guarantees

Status: complete.

Tasks:

* define the writer contract for line endings, raw header preservation,
  plus-line preservation, gzip output selection, flushing, and finish behavior;
* add round-trip tests for plain FASTQ records;
* add round-trip tests for gzip-compressed FASTQ records;
* add tests for write errors where practical;
* document when output compression is inferred from the filename extension.

Acceptance criteria:

* valid owned FASTQ records can be written and read back without record
  content changes;
* gzip output is finalized before command success is reported;
* writer failures surface as structured write errors with output path context.

Completion evidence:

* tightened `FastqWriter::finish` so plain output is flushed and gzip output is
  finalized and the wrapped buffered file flushed before success is reported;
* added plain FASTQ writer round-trip coverage proving owned record content,
  raw header comments, plus-line comments, and LF line endings are preserved;
* added gzip FASTQ writer round-trip coverage proving extension-selected gzip
  output has gzip framing, can be counted and parsed immediately after
  `finish`, and preserves records;
* added extension-policy coverage proving only the final `.gz` extension,
  case-insensitively, selects gzip output;
* added practical structured write-error coverage for writer creation failures
  with output path context;
* documented writer line-ending, preservation, extension-selection, flushing,
  gzip finalization, and structured error semantics in
  `docs/sphinx/native_fastq_core.rst`.

### M4.7 Migrate FASTQ-Side `subsample` And `enumerate` Consumers

Status: complete.

Tasks:

* route FASTQ-side `subsample` through the stable Milestone 4 reader and writer
  APIs;
* route plain FASTQ counting through the stable reader API;
* preserve `FASTQ.GZI`-assisted FASTQ.GZ enumeration behavior;
* preserve existing JSON payloads, examples, schemas, and command caveats
  unless a deliberate contract update is made in the same task;
* update command tests and documentation if behavior changes.

Acceptance criteria:

* FASTQ and FASTQ.GZ `subsample` behavior uses the Milestone 4 parser/writer
  contracts;
* `enumerate` remains sidecar-aware for FASTQ.GZ;
* public contracts remain stable or are deliberately versioned;
* relevant command tests and contract tests pass.

Completion evidence:

* exported the FASTQ gzip extension policy through the `crate::fastq` facade so
  command consumers can share the same case-insensitive final `.gz` decision as
  the reader and writer;
* routed `subsample` temporary output naming through that FASTQ gzip policy so
  staged `.FASTQ.GZ` outputs are still written with the M4 gzip writer before
  being renamed into place;
* preserved FASTQ-side `subsample` streaming through `open_fastq_reader`,
  `read_next_fastq_record`, `FastqRecord` identity bytes, and `FastqWriter`;
* preserved `enumerate` plain FASTQ counting through `count_fastq_records` and
  FASTQ.GZ counting through `FASTQ.GZI` sidecar creation/reuse;
* added command tests proving plain FASTQ `subsample` output round-trips
  through the stable FASTQ reader/writer path while preserving raw header and
  plus-line content;
* added command tests proving FASTQ.GZ `subsample` uses the case-insensitive M4
  gzip writer policy for uppercase `.FASTQ.GZ` output and can be counted and
  parsed through the stable FASTQ reader immediately after success;
* documented the `enumerate` and FASTQ-side `subsample` command-consumer
  boundaries in `docs/sphinx/native_fastq_core.rst`.

### M4.8 Migrate `consume`, Duplication, Deduplication, And Shard Consumers

Status: complete.

Tasks:

* route `consume` FASTQ and FASTQ.GZ import through the stable Milestone 4
  reader APIs while preserving threaded/import-label behavior;
* route `inspect_duplication` FASTQ scans through the stable reader API;
* route `deduplicate` FASTQ load/write paths through the stable reader/writer
  APIs;
* audit `explode` FASTQ.GZ shard planning and compression against the stable
  reader, writer, and `FASTQ.GZI` contracts;
* document any remaining richer or command-specific FASTQ behavior deferred
  beyond Milestone 4.

Acceptance criteria:

* selected FASTQ command consumers share the stable parser/writer primitives;
* JSON contracts and user-facing semantics remain stable unless explicitly
  updated;
* threaded consume behavior and `FASTQ.GZI`-guided execution remain covered by
  tests.

Completion evidence:

* audited unmapped `consume` and confirmed FASTQ/FASTQ.GZ imports use
  `open_fastq_reader_with_label`, `read_next_fastq_record`, logical input
  labels, threaded FASTQ.GZ conversion, and optional `FASTQ.GZI` totals for
  indexed worker-batch sizing;
* audited `inspect_duplication` and confirmed FASTQ/FASTQ.GZ scans use the
  stable reader API with parse failures mapped to `ParseUncertainty`;
* audited `deduplicate` and confirmed FASTQ/FASTQ.GZ load paths use the stable
  reader while applied writes use `write_fastq_records`;
* added the public `write_fastq_record_to` helper so command-specific FASTQ
  batch serializers can share the writer line contract without creating a file
  writer;
* routed FASTQ.GZ `explode` batch serialization through `write_fastq_record_to`
  while preserving `FASTQ.GZI` shard planning and concatenated gzip-member
  output;
* strengthened the FASTQ.GZ `explode` test to count and parse each shard
  through the stable reader and verify shard totals;
* strengthened the applied FASTQ `deduplicate` test to parse retained output
  through the stable reader instead of checking raw text only;
* added FASTQ.GZ `inspect_duplication` coverage through the stable reader path;
* retained existing indexed and threaded FASTQ.GZ `consume` tests:
  `unmapped_consume_uses_indexed_fastq_gz_input` and
  `unmapped_consume_parallelizes_multiple_fastq_gz_inputs`;
* documented the selected command-consumer boundaries and deferred richer
  command-specific behavior in `docs/sphinx/native_fastq_core.rst`.

### M4.9 Add FASTQ Oracle, Dependency Boundary, And Microbenchmarks

Status: complete.

Tasks:

* add native malformed FASTQ and FASTQ.GZ tests that do not depend on external
  parsers for expected failures;
* add any useful test-only oracle or differential coverage while keeping oracle
  usage outside production code;
* extend dependency-boundary tests so production FASTQ hot paths cannot import
  external generic bioinformatics parser crates;
* add or formalize FASTQ parse, FASTQ.GZ parse, and writer microbenchmark
  hooks with machine-readable output;
* document benchmark commands, profiles, schemas, and interpretation.

Acceptance criteria:

* malformed FASTQ and FASTQ.GZ behavior is covered by native tests;
* production FASTQ hot paths remain Bamana-native;
* FASTQ benchmarks are runnable without private data;
* benchmark JSON can be archived or validated by a documented schema.

Completion evidence:

* confirmed native malformed plain FASTQ coverage in `src/fastq/reader.rs`
  owns header marker, plus marker, blank read name, sequence/quality length,
  and truncated-record expectations without external parsers;
* added FASTQ.GZ malformed-record tests in `src/fastq/gzip.rs` proving valid
  gzip streams containing invalid FASTQ report native `InvalidFastq` errors for
  structural and truncated-record failures;
* extended `tests/contract/dependency_boundary.rs` so protected FASTQ modules
  and FASTQ-facing command consumers cannot directly import `noodles`, `bio`,
  `needletail`, `seq_io`, or `rust-htslib` parser crates;
* documented the native FASTQ oracle boundary in `docs/testing-oracles.md` and
  added a contract test requiring that policy language;
* added `src/bin/fastq_microbench.rs`, a deterministic synthetic FASTQ
  parser/writer microbenchmark covering plain parse, FASTQ.GZ parse, plain
  writer, gzip writer, and optional `enumerate` command timings;
* added `benchmarks/results/fastq_microbench.schema.json` for archivable
  machine-readable benchmark output;
* added `docs/sphinx/fastq_microbenchmarks.rst`, linked it from the Sphinx
  index, and updated benchmark result schema documentation;
* updated native FASTQ technical docs and the current milestone roadmap with
  oracle, dependency-boundary, and benchmark scope.
* focused malformed FASTQ.GZ tests passed with
  `cargo test fastq::gzip::tests::malformed_fastq_inside_valid_gzip_reports_native_fastq_error --lib`
  and
  `cargo test fastq::gzip::tests::truncated_fastq_record_inside_valid_gzip_reports_native_fastq_error --lib`;
* `cargo build --bin fastq_microbench` passed;
* FASTQ dependency-boundary and oracle-policy contract checks passed;
* `cargo run --bin fastq_microbench -- --profile small --iterations 1`
  passed and a JSON smoke check verified the benchmark name, profile,
  iteration count, generated record count, and expected result keys.

### M4.10 Close Milestone 4

Status: complete.

Tasks:

* run `cargo test`;
* run `cargo test --test contract`;
* run the FASTQ parser/writer microbenchmark or smoke benchmark profile;
* run the Sphinx documentation build;
* update `docs/roadmap/milestone-04-fastq.md`,
  `docs/roadmap/current_milestone.md`, README status text, Sphinx technical
  notes, and this task map with final Milestone 4 completion evidence;
* commit and push the closing milestone change.

Acceptance criteria:

* all M4.1 through M4.10 tasks are complete;
* full tests, contract tests, Sphinx, and FASTQ benchmark smoke checks pass;
* command consumer evidence is recorded for the selected FASTQ command paths;
* production FASTQ hot paths remain Bamana-native;
* Milestone 4 completion evidence is recorded in the repository.

Completion evidence:

* all M4.1 through M4.10 tasks are marked complete;
* `cargo test` passed with 207 library/unit tests, binary tests, 20 contract
  tests, 2 header-oracle tests, and doc tests;
* `cargo test --test contract` passed with 20 contract tests, including FASTQ
  dependency-boundary and oracle-policy checks;
* `cargo run --bin fastq_microbench -- --profile small --iterations 1` passed,
  and a JSON smoke check verified the benchmark name, profile, iteration count,
  1,024 generated records, and the expected parser/writer result keys;
* `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html` passed;
* updated `docs/roadmap/milestone-04-fastq.md` with final M4 closeout
  evidence, scope, verification commands, and deferred follow-up boundaries;
* updated `docs/roadmap/current_milestone.md` so Milestone 5 is active and
  Milestone 4 remains recorded as complete;
* updated `docs/roadmap.md` so Milestone 4 is complete and Milestone 5 is
  active;
* updated README status text and Sphinx FASTQ benchmark/native-core technical
  notes with M4 completion evidence.

## Milestone 5 Definition

Milestone 5 is complete only when the first proof commands have explicit
native-substrate evidence and no production `noodles` hot-path dependency.
The milestone covers `verify`, `header`, and `subsample` in that order.
`verify` and `header` are the proof commands for Milestone 1 BGZF plus
Milestone 2 header ownership. `subsample` is the first stronger end-to-end
proof across native BAM scanning, native FASTQ parsing, deterministic/random
selection, and native serialization or pass-through writing.

## Milestone 5 Current State

Status: complete as of 2026-05-20. Milestone 5 became active after Milestone 4
closed with stable native FASTQ/FASTQ.GZ parser and writer APIs, and closed
after proof-command migration evidence was recorded for `verify`, `header`,
and `subsample`.

Known present pieces:

* Milestone 1 native BGZF substrate is complete;
* Milestone 2 native BAM header codec is complete;
* Milestone 3 native BAM record scanner is complete for selected
  scanner-compatible consumers;
* Milestone 4 native FASTQ/FASTQ.GZ parser and writer APIs are complete;
* production `verify` already routes through native BGZF probing plus
  `parse_bam_header_from_native_bgzf`;
* production `header` already routes through native BGZF probing plus
  `parse_bam_header_from_native_bgzf`;
* BAM-side `subsample` streams through `BamScanner`, uses `BamRecordView` for
  filtering and selection identity, and writes retained scanner-owned raw
  record bytes;
* FASTQ and FASTQ.GZ `subsample` use `open_fastq_reader`,
  `read_next_fastq_record`, and `FastqWriter` from the native FASTQ core;
* contract dependency-boundary tests already protect production native header,
  verify, scanner, and migrated hot paths from direct `noodles` imports;
* `tests/contract/json_contract.rs` protects `benchmark`, `fastq`, and
  `unmap` as public contract commands with schemas, examples, and CLI docs;
* `header_microbench` can time `verify` and `header` command paths when passed
  a Bamana binary;
* `scanner_microbench` can emit `subsample_bam` command-level dry-run timing
  when passed a Bamana binary;
* `fastq_microbench` can emit `subsample_fastq` and `subsample_fastq_gz`
  command-level dry-run timings when passed a Bamana binary;
* production `subsample` already has governed JSON contracts for BAM, FASTQ,
  and FASTQ.GZ inputs and explicit deterministic/random selection policies.

Closeout evidence:

* `verify` is documented and tested as native BGZF plus native BAM header
  verification only;
* `header` is documented and tested as native BAM header extraction only;
* `subsample` is documented and tested as native BAM scanner traversal and
  native FASTQ/FASTQ.GZ parsing and writing for governed input formats;
* dependency-boundary tests name `verify`, `header`, and `subsample` as the M5
  proof-command set and keep direct production `noodles` usage isolated to
  CRAM compatibility;
* fixture and differential tests cover BAM, FASTQ, and FASTQ.GZ `subsample`
  selection and encounter-order evidence;
* proof-command smoke benchmark rows were recorded for `verify`, `header`,
  `subsample_bam`, `subsample_fastq`, and `subsample_fastq_gz`;
* M5 closeout verification completed with full tests, contract tests, Sphinx,
  and proof-command smoke benchmarks.

Milestone 5 closeout evidence must include:

* all M5.1 through M5.10 tasks complete;
* `verify` documented and tested as native BGZF plus native BAM header only;
* `header` documented and tested as native BAM header codec only;
* `subsample` documented and tested as using native BAM scanning and native
  FASTQ parsing/writing for its governed input formats;
* command JSON contracts remaining stable or deliberately versioned;
* differential, fixture, and contract tests passing for the migrated command
  paths;
* command-level benchmark evidence recorded for `verify`, `header`, and
  `subsample`;
* production `noodles` usage remaining isolated to CRAM compatibility, tests,
  oracles, and fixtures.

Command-surface scope:

* Milestone 5 evidence is limited to the proof-command migration set:
  `verify`, `header`, and `subsample`.
* Later command families such as `reheader`, `annotate_rg`,
  `inspect_duplication`, `deduplicate`, `forensic_inspect`, `sort`, `merge`,
  `explode`, `checksum`, and `consume` remain later waves unless a specific M5
  task explicitly includes them.

## Milestone 5 Task List

### M5.1 Activate Milestone 5 Scope And Baseline

Status: complete.

Tasks:

* update `docs/roadmap/current_milestone.md` so Milestone 5 is the active
  milestone only after Milestone 4 is complete;
* update roadmap/task-map status so Milestones 1 through 4 remain recorded
  with their correct completion state;
* audit `src/commands/verify.rs`, `src/commands/header.rs`,
  `src/commands/subsample.rs`, dependency-boundary tests, command contracts,
  and benchmark hooks;
* record which proof-command paths are already native and which still need
  migration;
* update README, CLI docs, Sphinx docs, and roadmap docs if the active
  milestone status changes visible project guidance.

Acceptance criteria:

* current milestone documentation names Milestone 5 as active only when M4 is
  closed;
* the task map records present proof-command evidence and gaps;
* no command behavior changes are made unless required by the baseline audit;
* public contracts for `benchmark`, `fastq`, and `unmap` remain protected.

Completion evidence:

* confirmed `docs/roadmap/current_milestone.md` names Milestone 5 as active
  only after Milestone 4 closeout and keeps Milestones 1 through 4 recorded as
  complete;
* audited `src/commands/verify.rs`: production `verify` probes input format,
  requires a BGZF-backed BAM container, and calls
  `parse_bam_header_from_native_bgzf`;
* audited `src/commands/header.rs`: production `header` follows the same
  native BGZF plus native BAM header parse path and returns `HeaderPayload`;
* audited `src/commands/subsample.rs`: FASTQ and FASTQ.GZ paths use
  `open_fastq_reader`, `read_next_fastq_record`, and `FastqWriter`, while BAM
  still uses `BamReader::open`, `parse_bam_header_from_reader`,
  `read_next_record_layout`, `serialize_record_layout`, and `BgzfWriter`
  rather than `BamScanner` or a scanner-owned raw-record bridge;
* audited `tests/contract/dependency_boundary.rs`: direct production
  `noodles` usage remains restricted to `src/ingest/cram.rs`, with separate
  checks for native header, verify, scanner, migrated BAM-record, and FASTQ
  hot paths;
* audited `tests/contract/json_contract.rs`, `spec/cli/commands.md`,
  `docs/cli.md`, schemas, and examples: public contract commands
  `benchmark`, `fastq`, and `unmap` remain protected;
* audited benchmark hooks: `header_microbench` can time `verify` and `header`
  when supplied a Bamana binary, and the benchmark framework exposes
  `subsample_only` workflow variants;
* updated `docs/roadmap/milestone-05-command-migration.md`,
  `docs/roadmap/current_milestone.md`, and Sphinx technical documentation with
  the M5.1 baseline audit;
* no command behavior changes were made.

### M5.2 Freeze Proof-Command Contracts And Fixtures

Status: complete.

Tasks:

* audit JSON schemas and success/failure examples for `verify`, `header`, and
  `subsample`;
* confirm CLI documentation describes the exact supported behavior for the
  proof commands;
* add or update fixtures for BAM, FASTQ, and FASTQ.GZ `subsample` coverage
  where gaps exist;
* record any intentional contract changes before migration work begins.

Acceptance criteria:

* proof-command schemas and examples exist and parse;
* contract tests fail if the governed proof-command documentation, schemas, or
  examples disappear;
* migration work has a stable before/after contract baseline.

Completion evidence:

* audited proof-command JSON schemas and canonical examples:
  `spec/jsonschema/verify.schema.json`,
  `spec/jsonschema/header.schema.json`,
  `spec/jsonschema/subsample.schema.json`,
  `spec/examples/verify.success.json`,
  `spec/examples/verify.failure.json`,
  `spec/examples/header.success.json`,
  `spec/examples/header.failure.json`,
  `spec/examples/subsample.success.json`,
  `spec/examples/subsample.success.deterministic.json`,
  `spec/examples/subsample.success.random.json`,
  `spec/examples/subsample.failure.json`, and
  `spec/examples/subsample.failure.invalid_fraction.json`;
* confirmed `spec/cli/commands.md`, `docs/cli.md`, and `docs/json-output.md`
  describe the supported `verify`, `header`, and `subsample` behavior and
  limits;
* added focused contract coverage in `tests/contract/json_contract.rs` so
  proof-command schemas, success/failure examples, CLI documentation, and
  JSON-output documentation cannot disappear silently;
* updated `tests/fixtures/manifest.json` so `subsample` has reserved BAM,
  FASTQ, FASTQ.GZ, malformed FASTQ, and truncated BAM fixture coverage through
  `tiny.clean.bam`, `tiny.clean.fastq`, `tiny.valid.fastq_gz`,
  `tiny.invalid.fastq.truncated`, and `tiny.invalid.bam.truncated_record`;
* updated Milestone 5 roadmap and Sphinx native command-migration notes with
  the frozen contract and fixture baseline;
* no intentional command contract changes were made before migration work.

### M5.3 Confirm And Harden `verify` Native Migration

Status: complete.

Tasks:

* confirm production `verify` uses native BGZF probing and native BAM header
  parsing only;
* add focused tests for valid BGZF BAM headers and representative malformed
  header failures if gaps remain;
* ensure `verify` documentation states that it is header-level verification,
  not full BAM payload validation or EOF checking;
* record command-level benchmark timing for `verify` where the benchmark
  framework supports it.

Acceptance criteria:

* `verify` has no production `noodles` dependency;
* `verify` checks remain limited and documented;
* JSON contracts remain stable;
* relevant command, contract, and dependency-boundary tests pass.

Completion evidence:

* confirmed production `src/commands/verify.rs` uses `probe_path` for shallow
  format/container recognition and `parse_bam_header_from_native_bgzf` for
  native BAM magic, header text, and binary reference-dictionary parsing;
* confirmed `tests/contract/dependency_boundary.rs` protects
  `src/commands/verify.rs` and the native BAM header path from direct
  production `noodles` imports;
* added `verify_accepts_header_without_bgzf_eof_marker` to document that
  `verify` remains header-level and does not perform EOF-marker checking;
* existing focused malformed-header tests continue to cover non-BGZF input,
  BGZF without BAM magic, negative `l_text`, and a truncated reference
  dictionary;
* confirmed README, `spec/cli/commands.md`, `docs/cli.md`,
  `docs/json-output.md`, and Sphinx docs state that `verify` is header-level
  verification, not alignment-record validation, full BAM body validation, or
  EOF checking;
* recorded command-level benchmark timing for `verify` through
  `target/debug/header_microbench --profile small --iterations 1
  --bamana-bin target/debug/bamana`, with the JSON smoke check confirming
  `benchmark: header_microbench`, `profile: small`, `iterations: 1`, and
  `verify` `ok_count: 1`;
* JSON contracts remained stable.

### M5.4 Confirm And Harden `header` Native Migration

Status: complete.

Tasks:

* confirm production `header` uses the native BAM header codec only;
* add focused tests for multi-member BGZF header parsing and malformed header
  failures if gaps remain;
* ensure `header` documentation preserves the boundary between header parsing
  and alignment-record validation;
* record command-level benchmark timing for `header` where the benchmark
  framework supports it.

Acceptance criteria:

* `header` has no production `noodles` dependency;
* `header` remains a header-only command;
* JSON contracts remain stable;
* relevant command, contract, and dependency-boundary tests pass.

Completion evidence:

* confirmed production `src/commands/header.rs` uses `probe_path` for shallow
  format/container recognition and `parse_bam_header_from_native_bgzf` for
  native BAM magic, header text, and binary reference-dictionary parsing;
* confirmed `tests/contract/dependency_boundary.rs` protects
  `src/commands/header.rs` and the native BAM header path from direct
  production `noodles` imports;
* existing focused tests continue to prove multi-member native BGZF header
  parsing and missing BAM magic failure behavior;
* added `header_command_does_not_validate_alignment_body` to document that
  `header` remains header-only and does not validate alignment records or the
  full BAM body;
* confirmed README, `spec/cli/commands.md`, `docs/cli.md`,
  `docs/json-output.md`, and Sphinx docs state that `header` parses only BAM
  header content and does not imply alignment-record validity, EOF presence, or
  full body readability;
* recorded command-level benchmark timing for `header` through
  `target/debug/header_microbench --profile small --iterations 1
  --bamana-bin target/debug/bamana`, with the JSON smoke check confirming
  `benchmark: header_microbench`, `profile: small`, `iterations: 1`, and
  `header` `ok_count: 1`;
* JSON contracts remained stable.

### M5.5 Migrate BAM-Side `subsample` To Native Scanner Or Raw-Record Bridge

Status: complete.

Tasks:

* replace BAM-side `subsample` use of `BamReader::open` and
  `read_next_record_layout` where practical with `BamScanner` or an explicit
  scanner-owned raw-record bridge;
* preserve deterministic and random selection semantics;
* preserve mapped-only and primary-only filtering semantics;
* preserve output record order and BAM serialization behavior;
* document any richer decode or serialization path that remains deliberately
  outside the M5 proof boundary.

Acceptance criteria:

* BAM-side `subsample` record traversal no longer depends on the older
  transitional reader path unless a documented bridge remains necessary;
* output BAM remains valid for covered fixtures;
* JSON contracts remain stable or are deliberately versioned;
* command tests cover representative random, deterministic, mapped-only, and
  primary-only paths.

Completion evidence:

* migrated BAM-side `subsample` from `BamReader::open` plus
  `read_next_record_layout` to `BamScanner::open` plus
  `BamScanner::next_record`;
* reused the scanner-owned native header for output header serialization;
* used `BamRecordView` for mapped-only and primary-only filtering;
* used `BamRecordView` read-name, sequence bytes, and raw record bytes for
  deterministic identity construction;
* wrote retained BAM records through the scanner-owned raw-record bridge via
  `BamRecordView::raw_record`, preserving encounter order and retained record
  bytes without reserializing `RecordLayout`;
* preserved seeded-random selection semantics and deterministic selection
  semantics;
* preserved BAM index invalidation reporting and existing JSON contract shape;
* added tests covering BAM header/output preservation, deterministic repeatable
  BAM subsampling, seeded-random repeatable BAM subsampling, and combined
  mapped-only plus primary-only filter accounting;
* documented the explicit scanner-owned raw-record bridge in the Milestone 5
  roadmap and Sphinx native command-migration notes.

### M5.6 Migrate FASTQ-Side `subsample` To Milestone 4 APIs

Status: complete.

Tasks:

* route FASTQ and FASTQ.GZ `subsample` through the stable M4 reader/writer
  APIs;
* preserve deterministic and random selection semantics;
* preserve identity modes that are valid for raw-read inputs;
* preserve output compression behavior for FASTQ.GZ outputs;
* ensure invalid BAM-only flags remain rejected for FASTQ inputs.

Acceptance criteria:

* FASTQ-side `subsample` consumes the stable native FASTQ parser/writer core;
* output FASTQ and FASTQ.GZ records preserve encounter order of retained
  records;
* JSON contracts remain stable or are deliberately versioned;
* command tests cover plain FASTQ and FASTQ.GZ paths.

Completion evidence:

* confirmed FASTQ and FASTQ.GZ `subsample` stream through the stable M4
  `open_fastq_reader` plus `read_next_fastq_record` APIs;
* confirmed retained FASTQ records are written through the stable M4
  `FastqWriter` API, including gzip-compressed output when the final output
  path uses a `.gz` suffix;
* preserved deterministic and seeded-random selection semantics;
* preserved raw-read deterministic identity semantics through
  `FastqRecord::identity_bytes`;
* preserved encounter-order output and M4 record fidelity for header comments,
  plus-line comments, sequence, and quality content;
* preserved rejection of BAM-only `--mapped-only`, `--primary-only`, and
  `--create-index` options before FASTQ streaming begins;
* added command tests covering the M4 reader/writer round trip, deterministic
  retained-record evidence, FASTQ.GZ writer policy, and FASTQ rejection for
  BAM-only flags;
* documented FASTQ-side `subsample` M4 API completion in the Milestone 5
  roadmap, current milestone notes, and Sphinx native command-migration notes.

### M5.7 Strengthen M5 Dependency Boundaries

Status: complete.

Tasks:

* extend dependency-boundary tests to name `verify`, `header`, and
  `subsample` as the M5 proof-command migration set;
* ensure production direct `noodles` imports remain limited to documented CRAM
  compatibility paths;
* document any test-only oracle usage for proof commands;
* fail tests if `noodles` is reintroduced into proof-command hot paths.

Acceptance criteria:

* dependency-boundary tests explicitly protect all three proof commands;
* any oracle usage is test-only and documented;
* production CRAM compatibility remains the only direct production `noodles`
  exception.

Completion evidence:

* added an explicit M5 proof-command hot-path boundary in
  `tests/contract/dependency_boundary.rs` naming `verify`, `header`, and
  `subsample`;
* protected the proof-command command files and native substrate paths for
  header parsing, BGZF reading, BAM scanning/writing, and FASTQ
  parsing/writing from direct `noodles` imports;
* preserved the global production dependency boundary that allows direct
  production `noodles` usage only in `src/ingest/cram.rs`;
* documented the M5 proof-command test-only oracle boundary in
  `docs/testing-oracles.md`;
* added contract coverage requiring that oracle policy to keep `verify`,
  `header`, and `subsample` expectations native-first;
* documented the strengthened dependency boundary in the Milestone 5 roadmap
  and Sphinx native command-migration notes.

### M5.8 Add Differential And Fixture Coverage For `subsample`

Status: complete.

Tasks:

* add or strengthen BAM `subsample` fixture coverage for deterministic and
  seeded-random selection;
* add or strengthen FASTQ and FASTQ.GZ `subsample` fixture coverage;
* add differential or checksum-style evidence proving retained records and
  encounter order are stable for covered inputs;
* ensure failure examples cover unsupported format, invalid fraction, and
  invalid filter combinations.

Acceptance criteria:

* `subsample` migration has fixture evidence across BAM, FASTQ, and FASTQ.GZ;
* retained record order and selection metadata are test-covered;
* governed examples remain stable.

Completion evidence:

* strengthened command tests for BAM deterministic and seeded-random
  `subsample` selection through the scanner-owned raw-record bridge;
* strengthened command tests for FASTQ deterministic and FASTQ.GZ seeded-random
  `subsample` selection through the Milestone 4 reader and writer;
* added ordered record-digest evidence for BAM, FASTQ, and FASTQ.GZ retained
  records, comparing command outputs back to source fixture records and proving
  encounter-order stability for the covered inputs;
* kept selection metadata covered by assertions that `order_preserved` remains
  true and retained counts match examined counts for full-retention fixture
  runs;
* added governed failure examples for unsupported format and invalid FASTQ
  filter/index combinations alongside the existing invalid-fraction example;
* added contract coverage requiring `subsample` failure examples to cover
  unsupported format, invalid fraction, and invalid filter combinations;
* documented the fixture and differential evidence in the Milestone 5 roadmap
  and Sphinx native command-migration notes.

### M5.9 Add Proof-Command Benchmark Evidence

Status: complete.

Tasks:

* add or formalize benchmark hooks for `verify`;
* add or formalize benchmark hooks for `header`;
* add or formalize benchmark hooks for BAM, FASTQ, and FASTQ.GZ `subsample`;
* record before/after or current native command-level deltas where practical;
* document benchmark interpretation and any unsupported comparator scope.

Acceptance criteria:

* each proof command has runnable benchmark or smoke benchmark coverage;
* benchmark output is machine-readable or archived through the existing
  benchmark framework;
* benchmark notes distinguish command-level timing from substrate-only timing.

Completion evidence:

* kept `verify` and `header` command-level smoke timings covered by
  `header_microbench --bamana-bin`;
* added `subsample_bam` command-level dry-run timing to
  `scanner_microbench --bamana-bin`;
* added `subsample_fastq` and `subsample_fastq_gz` command-level dry-run
  timings to `fastq_microbench --bamana-bin`;
* updated `scanner_microbench` and `fastq_microbench` result schemas so the new
  command timing rows are machine-readable and archivable;
* documented command-timing interpretation separately from in-process substrate
  timing in Sphinx and the Milestone 5 roadmap;
* ran proof-command benchmark smoke profiles with `--profile small
  --iterations 1 --bamana-bin target/debug/bamana`, confirming `verify: 1/1`,
  `header: 1/1`, `subsample_bam: 1/1`, `subsample_fastq: 1/1`, and
  `subsample_fastq_gz: 1/1`.

### M5.10 Close Milestone 5

Status: complete.

Tasks:

* run `cargo test`;
* run `cargo test --test contract`;
* run proof-command benchmark or smoke benchmark profiles;
* run the Sphinx documentation build;
* update `docs/roadmap/milestone-05-command-migration.md`,
  `docs/roadmap/current_milestone.md`, README status text, Sphinx technical
  notes, and this task map with final Milestone 5 completion evidence;
* commit and push the closing milestone change.

Acceptance criteria:

* all M5.1 through M5.10 tasks are complete;
* `verify`, `header`, and `subsample` are documented with native proof-command
  evidence;
* full tests, contract tests, Sphinx, and proof-command benchmark smoke checks
  pass;
* production `noodles` usage remains isolated to documented CRAM
  compatibility, tests, oracles, and fixtures;
* Milestone 5 completion evidence is recorded in the repository.

Completion evidence:

* closed Milestone 5 on 2026-05-20 with all M5.1 through M5.10 tasks marked
  complete;
* confirmed `verify`, `header`, and `subsample` have documented
  native-substrate proof-command evidence;
* confirmed production direct `noodles` usage remains isolated to documented
  CRAM compatibility, tests, oracles, and fixtures by the contract dependency
  boundary;
* preserved governed JSON contracts for `benchmark`, `fastq`, and `unmap` as
  public contract commands while closing the M5 proof-command set;
* reran proof-command smoke benchmarks with `--profile small --iterations 1
  --bamana-bin target/debug/bamana`, confirming `verify: 1/1`, `header: 1/1`,
  `subsample_bam: 1/1`, `subsample_fastq: 1/1`, and
  `subsample_fastq_gz: 1/1`;
* updated Milestone 5 roadmap, current milestone notes, README status text,
  Sphinx native command-migration notes, and this task map with final
  completion evidence;
* closeout verification completed with `cargo test`, `cargo test --test
  contract`, the Sphinx documentation build, and proof-command benchmark smoke
  profiles.

## Milestone 6 Definition

Milestone 6 is complete only when Bamana's first operational BAM inspection
and validation command wave is hardened on native substrates. The milestone
covers `check_eof`, `check_sort`, `check_map`, `summary`, `check_tag`, and
`validate`. It is the follow-on to Milestone 5: after proof commands establish
the native path, M6 makes the inspection commands dependable, well bounded,
benchmarked, and protected from production `noodles` hot-path regressions.

## Milestone 6 Current State

Status: complete as of 2026-05-21. Milestone 6 became active after Milestone 5
closed with proof-command migration, dependency-boundary, and benchmark
conventions recorded, and closed after M6.1 through M6.10 completed.

Completed pieces:

* `check_eof` already uses the native BGZF EOF-marker detection path;
* `check_sort` already uses `BamScanner` and scanner-owned field helpers for
  record traversal;
* `check_map` already preserves index-preferred behavior and uses
  `BamScanner` for scan fallback record traversal;
* `summary` already uses `BamScanner` for bounded and full record scans;
* `check_tag` already uses `BamScanner` plus record-view aux helpers for
  selected tag lookup;
* `validate` already uses `BamScanner` and `BamRecordView` for
  scanner-compatible record-level structural checks;
* scanner malformed-record tests and dependency-boundary tests already protect
  the scanner substrate and selected migrated hot paths;
* `spec/cli/commands.md`, `docs/cli.md`, README, JSON schemas, and success and
  failure examples describe the six M6 commands as governed public surfaces;
* bounded versus full-scan claims, index-derived versus scan-derived evidence,
  and validation caveats are documented and test-covered;
* `validate` is documented and tested as structural/internal-consistency
  validation only;
* command-level benchmark smoke evidence is recorded for the full M6
  inspection wave;
* dependency-boundary tests name `check_eof`, `check_sort`, `check_map`,
  `summary`, `check_tag`, and `validate` as one protected milestone set.

Milestone 6 closeout evidence must include:

* all M6.1 through M6.10 tasks complete;
* `check_eof` documented and tested as the native BGZF EOF-marker command;
* `check_sort`, `check_map`, `summary`, `check_tag`, and `validate`
  documented and tested as native scanner/header consumers for their
  scanner-compatible paths;
* command JSON contracts remaining stable or deliberately versioned;
* fixture coverage for bounded scan behavior, full scan behavior, malformed
  inputs, absent evidence, and index-versus-scan distinctions where relevant;
* command-level benchmark or smoke benchmark evidence for the M6 command set;
* production `noodles` usage remaining isolated to CRAM compatibility, tests,
  oracles, and fixtures.

Command-surface scope:

* Milestone 6 evidence is limited to inspection and validation commands:
  `check_eof`, `check_sort`, `check_map`, `summary`, `check_tag`, and
  `validate`.
* Mutation, rewrite, normalization, deduplication, forensic, checksum, sort,
  merge, explode, and ingest families remain later waves unless a specific M6
  task explicitly includes them.

## Milestone 6 Task List

### M6.1 Activate Milestone 6 Scope And Baseline

Status: complete.

Tasks:

* update `docs/roadmap/current_milestone.md` so Milestone 6 is the active
  milestone only after Milestone 5 is complete;
* update roadmap/task-map status so Milestones 1 through 5 remain recorded
  with their correct completion state;
* audit `check_eof`, `check_sort`, `check_map`, `summary`, `check_tag`, and
  `validate` command implementations, tests, contracts, examples, and docs;
* record which command paths already use native BGZF/header/scanner primitives
  and which still need hardening;
* update README, CLI docs, Sphinx docs, and roadmap docs if the active
  milestone status changes visible project guidance.

Acceptance criteria:

* current milestone documentation names Milestone 6 as active only when M5 is
  closed;
* the task map records present M6 command evidence and gaps;
* no command behavior changes are made unless required by the baseline audit;
* public contract commands remain explicitly protected.

Completion evidence:

* updated `docs/roadmap/current_milestone.md` so Milestone 6 is active only
  after the recorded Milestone 5 closeout;
* updated `docs/roadmap.md`, `docs/roadmap/milestone-06-inspection-validation.md`,
  README status text, and Sphinx technical documentation so Milestone 6 is the
  active native-core milestone;
* confirmed Milestones 1 through 5 remain recorded as complete;
* audited `src/commands/check_eof.rs`: production `check_eof` probes BAM/BGZF
  input and uses native `bgzf::has_bgzf_eof`;
* audited `src/commands/check_sort.rs`: production `check_sort` uses
  `BamScanner` and `BamRecordView` fields for ordering evidence;
* audited `src/commands/check_map.rs`: production `check_map` preserves
  usable BAI index-derived evidence and falls back to `BamScanner` traversal;
* audited `src/commands/summary.rs`: production `summary` uses native header
  metadata, optional BAI-derived totals, and bounded or full `BamScanner`
  scans;
* audited `src/commands/check_tag.rs`: production `check_tag` uses
  `BamScanner` plus native aux traversal helpers for selected tag lookup;
* audited `src/commands/validate.rs`: production `validate` routes through the
  native validation substrate built on native header parsing, `BamScanner`, and
  `BamRecordView`;
* audited `tests/contract/dependency_boundary.rs`: scanner substrate and
  selected migrated hot paths are already protected from direct `noodles`
  imports, while a single named M6 command-set boundary remains planned for
  M6.9;
* audited `spec/cli/commands.md`, `docs/cli.md`, README, schemas, and examples:
  baseline documentation exists for all six M6 commands, with detailed contract
  freezing left to M6.2;
* no command behavior changes were made.

### M6.2 Freeze Inspection Command Contracts And Examples

Status: complete.

Tasks:

* audit JSON schemas and success/failure examples for `check_eof`,
  `check_sort`, `check_map`, `summary`, `check_tag`, and `validate`;
* confirm CLI documentation describes each command's bounded and full-scan
  semantics accurately;
* add or update fixtures for malformed BAM, missing EOF, sorted/unsorted BAM,
  mapped/unmapped evidence, absent tags, and validation failures where gaps
  exist;
* record any intentional contract changes before implementation hardening.

Acceptance criteria:

* inspection command schemas and examples exist and parse;
* contract tests fail if governed M6 command documentation, schemas, or
  examples disappear;
* bounded evidence and absence claims are documented without overclaiming.

Completion evidence:

* audited JSON schemas and canonical success/failure examples for `check_eof`,
  `check_sort`, `check_map`, `summary`, `check_tag`, and `validate`;
* confirmed all six M6 commands are documented in `spec/cli/commands.md`,
  `docs/cli.md`, README, and `docs/json-output.md`;
* added contract coverage requiring each M6 command to keep a schema, canonical
  success example, canonical failure example, CLI contract docs, user-facing
  CLI docs, and JSON-output docs;
* confirmed bounded/full-scan language is present for `check_sort`,
  `check_map`, `summary`, `check_tag`, and `validate`, and EOF-only language is
  present for `check_eof`;
* updated fixture planning with explicit M6 assets for missing EOF,
  sorted/queryname/unsorted BAMs, mapped/unmapped evidence, index-derived
  mapping and summary evidence, observed and absent tag evidence, malformed aux
  evidence, and structural validation failures;
* recorded the M6.2 contract freeze in the Milestone 6 roadmap and Sphinx
  technical documentation;
* no intentional command behavior or JSON shape changes were made.

### M6.3 Harden `check_eof` Native BGZF Boundary

Status: complete.

Tasks:

* confirm `check_eof` uses the native BGZF EOF-marker path only;
* add tests for present EOF marker, missing EOF marker, truncated tail, and
  non-BGZF inputs if coverage gaps remain;
* ensure docs state that `check_eof` does not imply BAM header validity or
  alignment-record validity;
* add command-level smoke timing where benchmark hooks support it.

Acceptance criteria:

* `check_eof` has no production `noodles` dependency;
* success and failure payloads remain stable or are deliberately versioned;
* docs preserve the narrow EOF-only command boundary.

Completion evidence:

* confirmed production `src/commands/check_eof.rs` uses shallow probing plus
  native `bgzf::has_bgzf_eof` and does not parse the BAM header or alignment
  records;
* added focused command tests for present canonical EOF marker, missing EOF
  marker, tail too short for the canonical EOF marker, non-BGZF BAM input, and
  invalid BAM payload bytes with a present EOF marker;
* documented that `check_eof` success does not imply BAM magic validity, BAM
  header validity, alignment-record validity, or auxiliary-field validity;
* recorded the M6.3 boundary in the Milestone 6 roadmap and Sphinx technical
  documentation;
* confirmed command-level smoke timing for `check_eof` through
  `bgzf_microbench --bamana-bin`, with the M6.3 smoke run reporting
  `check_eof: 1/1`.

### M6.4 Harden `check_sort` Scanner Evidence

Status: complete.

Tasks:

* audit `check_sort` native scanner traversal and strict/bounded scan behavior;
* strengthen tests for coordinate sort, queryname sort, specialized sort,
  unknown sort order, bounded scan caveats, and strict scan failures where
  needed;
* ensure docs state that `check_sort` is not full BAM validation;
* add command-level smoke timing where benchmark hooks support it.

Acceptance criteria:

* `check_sort` remains scanner-backed for record traversal;
* JSON contracts remain stable or are deliberately versioned;
* bounded and strict modes are test-covered and documented.

Completion evidence:

* confirmed production `src/commands/check_sort.rs` opens BAM input through
  `BamScanner` and derives ordering evidence from `BamRecordView` coordinates,
  flags, and read names;
* strengthened tests for coordinate sort, queryname sort,
  `template-coordinate` specialized sort reporting, unknown declared sort
  order, bounded-scan caveats, and strict violation detection after a bounded
  sample window;
* documented that bounded `check_sort` evidence is scoped to examined records
  and that `--strict` expands sequential inspection without becoming full BAM
  structural validation;
* added `check_sort` command smoke timing to `scanner_microbench --bamana-bin`
  and updated its machine-readable schema and documentation;
* recorded the M6.4 scanner-evidence boundary in the Milestone 6 roadmap and
  Sphinx technical documentation;
* confirmed the M6.4 smoke run reported `check_sort: 1/1`.

### M6.5 Harden `check_map` Index And Scanner Evidence

Status: complete.

Tasks:

* audit `check_map` index-preferred behavior and scanner fallback behavior;
* strengthen tests for usable index summaries, missing/unusable indices, scan
  fallback, mapped/unmapped counts, and bounded caveats where needed;
* ensure docs distinguish index-derived evidence from scan-derived evidence;
* add command-level smoke timing where benchmark hooks support it.

Acceptance criteria:

* `check_map` preserves index-preferred behavior where documented;
* scanner fallback remains native and bounded as documented;
* payloads identify evidence source and limitations accurately.

Completion evidence:

* audited `src/commands/check_map.rs`: production behavior still opens BAM via
  `BamScanner`, prefers usable BAI metadata when requested, and falls back to
  scanner-derived record evidence when index evidence is absent, incomplete,
  unsupported, malformed, or disabled;
* strengthened command tests for usable BAI summaries, missing indexes,
  incomplete BAI metadata, invalid BAI fallback, explicit index disabling,
  mapped/unmapped scan counts, bounded-scan caveats, and full-scan evidence
  beyond the bounded window;
* documented the distinction between index-derived and scan-derived
  `check_map` evidence in CLI, JSON-output, Sphinx, and roadmap sources;
* added `check_map` command-level timing to `scanner_microbench --bamana-bin`
  and updated the benchmark schema, benchmark documentation, and contract test
  guard;
* confirmed M6.5 smoke run reported `check_map: 1/1`.

### M6.6 Harden `summary` Scanner Evidence

Status: complete.

Tasks:

* audit `summary` bounded and full scan paths;
* strengthen tests for header-only summaries, index-assisted summaries,
  bounded scans, full scans, malformed records, and empty BAM bodies where
  needed;
* ensure docs state that `summary` is operational evidence, not full
  validation;
* add command-level smoke timing where benchmark hooks support it.

Acceptance criteria:

* `summary` uses native header/scanner primitives for record evidence;
* bounded/full scan distinctions remain visible in output;
* JSON contracts remain stable or are deliberately versioned.

Completion evidence:

* audited `src/commands/summary.rs`: production behavior opens BAM via
  `BamScanner`, uses native header metadata, optionally parses adjacent BAI
  metadata, and builds scan-derived counts through `SummaryAccumulator`;
* strengthened command tests for header-only BAM summaries, bounded scanner
  evidence, full scanner evidence, BAI-assisted summaries, malformed alignment
  record failures, and separation of index-derived totals from scan-derived
  counts;
* documented that `summary` is operational evidence rather than full BAM
  validation in CLI, JSON-output, Sphinx, and roadmap sources;
* confirmed `scanner_microbench --bamana-bin` already emits a `summary`
  command timing row and added a contract guard for that row;
* confirmed M6.6 smoke run reported `summary: 1/1`.

### M6.7 Harden `check_tag` Aux Traversal Evidence

Status: complete.

Tasks:

* audit `check_tag` scanner-owned aux traversal;
* strengthen tests for present tag, absent tag under bounded scan, absent tag
  under complete scan, duplicate tags, malformed aux payloads, and unsupported
  aux shapes where needed;
* ensure docs state that tag value semantics are limited to traversal and
  supported extraction behavior;
* add command-level smoke timing where benchmark hooks support it.

Acceptance criteria:

* `check_tag` uses scanner-owned aux traversal;
* absent-tag claims distinguish bounded from complete scans;
* malformed aux failures remain structured and precise.

Completion evidence:

* audited `src/commands/check_tag.rs`: production behavior opens BAM via
  `BamScanner`, traverses borrowed `BamRecordView` auxiliary bytes through
  `record_aux_contains_tag`, and returns structured indeterminate payloads for
  aux traversal uncertainty;
* strengthened command tests for present tags, bounded absent tags,
  complete-scan absent tags, type-filter mismatches, duplicate tag records,
  malformed aux payloads, and unsupported B-array shapes;
* documented the `check_tag` evidence boundary in CLI, JSON-output, Sphinx, and
  roadmap sources, including record-level hit counting and limited tag value
  semantics;
* confirmed `scanner_microbench --bamana-bin` already emits a `check_tag`
  command timing row and added a contract guard for that row;
* confirmed M6.7 smoke run reported `check_tag: 1/1`.

### M6.8 Harden `validate` Structural Validation Boundary

Status: complete.

Tasks:

* audit `validate` scanner-backed record structural checks;
* strengthen tests for malformed record structure, aux traversal failures,
  bounded validation, severity reporting, and clean minimal BAM inputs where
  needed;
* ensure docs state that `validate` does not imply biological correctness,
  reference concordance, or full optional-field semantic validation;
* add command-level smoke timing where benchmark hooks support it.

Acceptance criteria:

* `validate` remains native for scanner-compatible record traversal;
* structural findings are precise and severity-coded;
* public docs and payloads avoid overclaiming validation scope.

Completion evidence:

* audited `src/commands/validate.rs` and `src/bam/validate.rs`: production
  behavior probes BAM/BGZF input, opens BAM via `BamScanner`, validates
  scanner-compatible records through borrowed `BamRecordView` fields, and uses
  native auxiliary traversal for aux structural findings;
* strengthened validation tests for clean minimal BAM input, header-only scope,
  bounded-record scope, malformed record structure, malformed aux traversal,
  severity-coded findings, and configured finding-list limits;
* documented that `validate` is structural/internal-consistency validation and
  does not imply biological correctness, external reference concordance, or
  complete optional-field semantic validation in CLI, JSON-output, Sphinx, and
  roadmap sources;
* confirmed `scanner_microbench --bamana-bin` already emits a `validate`
  command timing row and added a contract guard for that row;
* confirmed M6.8 smoke run reported `validate: 1/1`.

### M6.9 Strengthen M6 Dependency Boundaries And Benchmarks

Status: complete.

Tasks:

* extend dependency-boundary tests to name the M6 inspection command set;
* ensure production direct `noodles` imports remain limited to documented CRAM
  compatibility paths;
* add or formalize command-level benchmark hooks for the M6 command set;
* record benchmark interpretation notes that distinguish substrate timing,
  command startup, bounded scans, full scans, index use, and malformed-input
  behavior.

Acceptance criteria:

* dependency-boundary tests explicitly protect the M6 command set;
* every M6 command has runnable smoke benchmark or timing evidence;
* benchmark notes are documented and do not imply comparator parity where none
  exists.

Completion evidence:

* added `M6_INSPECTION_COMMAND_HOT_PATHS` to
  `tests/contract/dependency_boundary.rs`, explicitly naming `check_eof`,
  `check_sort`, `check_map`, `summary`, `check_tag`, and `validate` plus their
  protected native substrate paths;
* strengthened benchmark schema contract coverage so `check_eof` is guarded in
  `bgzf_microbench.schema.json` and `summary`, `check_sort`, `check_map`,
  `check_tag`, and `validate` are guarded in
  `scanner_microbench.schema.json`;
* documented benchmark interpretation notes for substrate timing versus command
  startup, bounded/full scan behavior, index absence in scanner smoke fixtures,
  malformed-input limitations, and lack of external comparator parity;
* added runtime benchmark notes to `bgzf_microbench` and `scanner_microbench`
  JSON output;
* confirmed M6.9 smoke runs reported `check_eof: 1/1` and scanner command rows
  `summary: 1/1`, `check_sort: 1/1`, `check_map: 1/1`, `validate: 1/1`, and
  `check_tag: 1/1`.

### M6.10 Close Milestone 6

Status: complete.

Tasks:

* run `cargo test`;
* run `cargo test --test contract`;
* run M6 command benchmark or smoke benchmark profiles;
* run the Sphinx documentation build;
* update `docs/roadmap/milestone-06-inspection-validation.md`,
  `docs/roadmap/current_milestone.md`, README status text, Sphinx technical
  notes, and this task map with final Milestone 6 completion evidence;
* commit and push the closing milestone change.

Acceptance criteria:

* all M6.1 through M6.10 tasks are complete;
* `check_eof`, `check_sort`, `check_map`, `summary`, `check_tag`, and
  `validate` are documented with native inspection/validation evidence;
* full tests, contract tests, Sphinx, and M6 command benchmark smoke checks
  pass;
* production `noodles` usage remains isolated to documented CRAM
  compatibility, tests, oracles, and fixtures;
* Milestone 6 completion evidence is recorded in the repository.

Completion evidence:

* ran `cargo test`;
* ran `cargo test --test contract`;
* ran `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html`;
* ran `cargo build --bin bamana --bin bgzf_microbench --bin
  scanner_microbench`;
* ran M6 command smoke profiles:
  `bgzf_microbench --profile small --iterations 1 --bamana-bin
  target/debug/bamana` reported `verify:1/1, check_eof:1/1`, and
  `scanner_microbench --profile small --iterations 1 --bamana-bin
  target/debug/bamana` reported `summary:1/1, check_sort:1/1`,
  `check_map:1/1`, `validate:1/1`, `check_tag:1/1`, and
  `subsample_bam:1/1`;
* updated Milestone 6 roadmap, current milestone notes, README status text,
  Sphinx technical notes, and this task map with final Milestone 6 completion
  evidence;
* confirmed production direct `noodles` usage remains isolated to documented
  CRAM compatibility through contract tests;
* Milestone 6 is complete as of 2026-05-21, and Milestone 7 became active on
  2026-05-22 after that closeout was recorded.

## Milestone 7 Definition

Milestone 7 is complete only when Bamana's native mutation, conservative
remediation, and provenance-inspection command wave is hardened on native
substrates. The milestone covers `reheader`, `annotate_rg`,
`inspect_duplication`, `deduplicate`, and `forensic_inspect`. It follows the
inspection/validation wave because these commands have higher blast radius:
they either mutate output, remediate duplicated collection blocks, or assemble
operational provenance evidence.

## Milestone 7 Current State

Status: complete as of 2026-05-22. Milestone 7 became active only after
Milestone 6 closed on 2026-05-21, and closed after M7.1 through M7.10
completed on 2026-05-22.

Known present pieces:

* Milestone 2 native BAM header codec provides the substrate for `reheader`,
  `annotate_rg`, checksum-domain header serialization, and header evidence in
  forensic reports;
* Milestone 3 native BAM scanner already backs BAM-body scans in
  `inspect_duplication` and `forensic_inspect`;
* Milestone 4 stabilized native FASTQ/FASTQ.GZ parser and writer APIs used by
  FASTQ-side duplication and deduplication paths;
* `reheader` command orchestration already separates header-only mutation from
  record-level `RG:Z` annotation and exposes dry-run, rewrite, reindex, and
  checksum verification controls;
* `reheader` uses the native header codec, native header serialization, native
  record-layout serialization, and native BGZF writer for current rewrite
  paths;
* `annotate_rg` command orchestration already enforces explicit record-mode and
  header-policy choices;
* `annotate_rg` uses the native header codec, aux-tag traversal,
  record-layout serialization, and native BGZF writer for safe rewrites;
* `inspect_duplication` already supports BAM, FASTQ, and FASTQ.GZ inputs and
  uses `BamScanner` plus native FASTQ helpers for current scan paths;
* `deduplicate` already implements conservative remediation modes for BAM,
  FASTQ, and FASTQ.GZ inputs, with dry-run and applied modes;
* FASTQ-side `deduplicate` uses the native FASTQ reader and writer, and M7.6
  moved BAM-side `deduplicate` planning onto `BamScanner` plus
  `BamRecordView::to_record_layout` while preserving native header
  serialization, record-layout serialization, and BGZF writer primitives;
* `forensic_inspect` already uses scanner-backed BAM body evidence for
  read-group, read-name, aux-tag, and duplication-hallmark checks.

Closed gaps:

* M7 command contracts and examples were audited for mutation safety,
  dry-run/apply distinctions, remediation limits, and forensic caveats;
* command-level benchmark smoke evidence is recorded for the full M7 command
  set;
* dependency-boundary tests name `reheader`, `annotate_rg`,
  `inspect_duplication`, `deduplicate`, and `forensic_inspect` as one
  protected milestone set.

Milestone 7 closeout evidence must include:

* all M7.1 through M7.10 tasks complete;
* `reheader` documented and tested as native header-only mutation planning and
  execution;
* `annotate_rg` documented and tested as native record-level read-group
  annotation distinct from `reheader`;
* `inspect_duplication` documented and tested as native BAM/FASTQ collection
  duplication inspection, not biological duplicate marking;
* `deduplicate` documented and tested as conservative remediation over native
  BAM/FASTQ paths, not broad duplicate collapse;
* `forensic_inspect` documented and tested as evidence-driven provenance
  inspection, not fraud detection;
* command JSON contracts remaining stable or deliberately versioned;
* command-level benchmark or smoke benchmark evidence for the M7 command set;
* production `noodles` usage remaining isolated to CRAM compatibility, tests,
  oracles, and fixtures.

Command-surface scope:

* Milestone 7 evidence is limited to mutation, remediation, and forensics
  commands: `reheader`, `annotate_rg`, `inspect_duplication`, `deduplicate`,
  and `forensic_inspect`.
* Large transform, ordering, merge, checksum, explode, and ingest families
  remain later waves unless a specific M7 task explicitly includes them.

## Milestone 7 Task List

### M7.1 Activate Milestone 7 Scope And Baseline

Status: complete.

Tasks:

* update `docs/roadmap/current_milestone.md` so Milestone 7 is the active
  milestone only after Milestone 6 is complete;
* update roadmap/task-map status so Milestones 1 through 6 remain recorded
  with their correct completion state;
* audit `reheader`, `annotate_rg`, `inspect_duplication`, `deduplicate`, and
  `forensic_inspect` command implementations, helper modules, tests,
  contracts, examples, and docs;
* record which command paths already use native header, scanner, FASTQ, and
  writer primitives and which still need hardening;
* update README, CLI docs, Sphinx docs, and roadmap docs if the active
  milestone status changes visible project guidance.

Acceptance criteria:

* current milestone documentation names Milestone 7 as active only when M6 is
  closed;
* the task map records present M7 command evidence and gaps;
* no command behavior changes are made unless required by the baseline audit;
* public contract commands remain explicitly protected.

Completion evidence:

* confirmed Milestone 6 is recorded complete and Milestone 7 is active only
  after the 2026-05-21 M6 closeout;
* audited `reheader`, `annotate_rg`, `inspect_duplication`, `deduplicate`, and
  `forensic_inspect` command and helper paths;
* recorded that `reheader` and `annotate_rg` use native header, aux-tag,
  record-layout, checksum-domain, and BGZF writer primitives while preserving
  their header-only versus record-touching distinction;
* recorded that `inspect_duplication` and `forensic_inspect` use `BamScanner`
  for BAM body evidence, and that `inspect_duplication` plus `deduplicate` use
  native FASTQ reader/writer paths for FASTQ and FASTQ.GZ inputs;
* recorded the remaining BAM `deduplicate` hardening gap around
  scanner-owned raw-record or writer-bridge loading;
* updated README status text, current milestone notes, the Milestone 7 roadmap,
  Sphinx technical notes, and this task map without changing command behavior.

### M7.2 Freeze Mutation And Forensics Contracts And Examples

Status: complete.

Tasks:

* audit JSON schemas and success/failure examples for `reheader`,
  `annotate_rg`, `inspect_duplication`, `deduplicate`, and
  `forensic_inspect`;
* confirm CLI documentation describes dry-run versus applied behavior,
  mutation safety, remediation limits, and forensic caveats accurately;
* add or update fixtures for header mutation, RG annotation, clean duplication
  scans, duplicated collection blocks, remediation plans, applied remediation,
  and forensic provenance evidence where gaps exist;
* record any intentional contract changes before implementation hardening.

Acceptance criteria:

* M7 command schemas and examples exist and parse;
* contract tests fail if governed M7 command documentation, schemas, or
  examples disappear;
* mutation, remediation, and forensic claims are documented without
  overclaiming.

Completion evidence:

* audited the JSON schemas and canonical success/failure examples for
  `reheader`, `annotate_rg`, `inspect_duplication`, `deduplicate`, and
  `forensic_inspect`;
* added contract tests that require all five M7 commands to keep schemas,
  success/failure examples, CLI docs, JSON-output docs, README coverage, and
  Sphinx mutation/forensics notes;
* added dependency-boundary contract coverage that names the five M7 command
  paths and keeps their mutation, remediation, and forensics hot paths free of
  direct production `noodles` imports;
* reserved expected-output fixture documentation for `reheader` and
  `annotate_rg`, and updated the fixture manifest to map the clean BAM baseline
  to those mutation commands;
* confirmed duplication and forensics fixture reservations cover clean,
  duplicated FASTQ/BAM, and provenance-anomaly scenarios without expanding
  biological duplicate-marking or fraud-detection claims;
* kept command behavior unchanged while freezing the governed M7 contract
  surface for later implementation hardening.

### M7.3 Harden `reheader` Native Header Mutation Boundary

Status: complete.

Tasks:

* audit `reheader` use of native header parsing and serialization;
* strengthen tests for full header replacement, `@RG` mutation, `@PG`
  mutation, comment mutation, dry-run planning, rewrite modes, checksum
  verification, and index invalidation where needed;
* ensure docs state that `reheader` is header-only and does not modify
  per-record `RG:Z` tags;
* add command-level smoke timing where benchmark hooks support it.

Acceptance criteria:

* `reheader` remains native-header backed and header-only;
* JSON contracts remain stable or are deliberately versioned;
* true in-place or rewrite safety claims are only made where proven.

Completion evidence:

* audited `src/bam/reheader.rs` and confirmed the command remains backed by
  native BAM header parsing, native SAM-style header serialization, native
  record-layout serialization, and the native BGZF writer;
* added `reheader` unit coverage for dry-run planning across `@RG`, `@PG`, and
  `@CO` mutations, conservative in-place planning, rewrite execution, full
  header replacement, checksum verification with header bytes excluded, BAM
  index invalidation reporting, and preservation of per-record `RG:Z` bytes;
* added a failing test for unproven true in-place execution without rewrite
  fallback so true in-place safety is not overclaimed;
* extended `header_microbench --bamana-bin` command smoke timings to include a
  `reheader` dry-run path and updated the benchmark result schema and Sphinx
  benchmark documentation;
* kept the governed `reheader` JSON contract stable while adding hardening
  tests and benchmark smoke coverage.

### M7.4 Harden `annotate_rg` Native Record Annotation Boundary

Status: complete.

Tasks:

* audit `annotate_rg` use of native header and record mutation primitives;
* strengthen tests for only-missing, replace-existing, fail-on-conflict,
  require-header-rg, create-header-rg, add-header-rg, set-header-rg, dry-run,
  checksum verification, and index invalidation where needed;
* ensure docs preserve the distinction between record-level `annotate_rg` and
  header-only `reheader`;
* add command-level smoke timing where benchmark hooks support it.

Acceptance criteria:

* `annotate_rg` remains explicit about record-touching behavior;
* record and header policies are test-covered;
* JSON contracts remain stable or are deliberately versioned.

Completion evidence:

* audited `src/bam/annotate_rg.rs` and confirmed the command remains backed by
  native BAM header parsing, native SAM-style header serialization, native
  aux-tag traversal, native record-layout serialization, and the native BGZF
  writer;
* added `annotate_rg` unit coverage for `only_missing`, `replace_existing`,
  `fail_on_conflict`, `require_header_rg`, `create_header_rg`,
  `add_header_rg`, `set_header_rg`, dry-run planning, checksum verification
  with `RG` excluded, thread-count caveat reporting, and BAM index
  invalidation reporting;
* added record-output assertions proving `annotate_rg` mutates per-record
  `RG:Z` tags while remaining distinct from header-only `reheader`;
* extended `header_microbench --bamana-bin` command smoke timings to include an
  `annotate_rg` dry-run path and updated the benchmark result schema and
  Sphinx benchmark documentation;
* kept the governed `annotate_rg` JSON contract stable while adding hardening
  tests and benchmark smoke coverage.

### M7.5 Harden `inspect_duplication` Native Scan Boundary

Status: complete.

Tasks:

* audit BAM, FASTQ, and FASTQ.GZ duplication inspection paths against native
  scanner and Milestone 4 FASTQ APIs;
* strengthen tests for clean inputs, whole-file append signatures, local block
  signatures, identity modes, sample-record bounds, full-scan behavior,
  malformed inputs, and RG-mode rejection for FASTQ where needed;
* ensure docs state that `inspect_duplication` is collection-duplication and
  operator-error inspection, not molecular duplicate marking;
* add command-level smoke timing where benchmark hooks support it.

Acceptance criteria:

* `inspect_duplication` uses native scanner/FASTQ paths for supported inputs;
* identity modes and scan bounds remain explicit in output;
* JSON contracts remain stable or are deliberately versioned.

Completion evidence:

* audited `src/forensics/duplication.rs` and confirmed BAM inspection uses
  `BamScanner`, borrowed native record views, native sequence/quality decoding,
  and native aux-tag traversal, while FASTQ and FASTQ.GZ inspection uses the
  Milestone 4 `open_fastq_reader` and `read_next_fastq_record` APIs;
* strengthened `inspect_duplication` unit coverage for clean full scans,
  whole-file append signatures, local contiguous-block signatures, bounded
  scan caveat reporting, malformed FASTQ parse uncertainty with partial
  payload evidence, BAM read-group identity semantics, and FASTQ rejection of
  BAM-only `qname_seq_qual_rg` identity mode;
* confirmed output remains explicit about `identity_mode`, `scan_mode`,
  `records_examined`, collection-duplication/operator-error scope, and the
  non-use of BAM duplicate flags as primary evidence;
* extended `scanner_microbench --bamana-bin` command smoke timings to include
  a full `inspect_duplication --identity qname_seq_qual_rg` scan and updated
  the benchmark result schema plus Sphinx benchmark documentation;
* kept the governed `inspect_duplication` JSON contract stable while adding
  hardening tests and benchmark smoke coverage.

### M7.6 Harden `deduplicate` Conservative Remediation Boundary

Status: complete.

Tasks:

* audit BAM, FASTQ, and FASTQ.GZ deduplication load/write paths against native
  scanner, raw-record, header, and Milestone 4 FASTQ APIs;
* migrate BAM paths away from older transitional reader/layout usage where a
  scanner-owned raw-record or native writer bridge can preserve behavior;
* strengthen tests for dry-run plans, applied remediation, keep-first/keep-last
  policy, whole-file append, contiguous-block, global-exact mode, emitted
  removed-record reports, checksum verification, and malformed inputs where
  needed;
* ensure docs state that `deduplicate` is conservative remediation, not
  biological duplicate collapse;
* add command-level smoke timing where benchmark hooks support it.

Acceptance criteria:

* `deduplicate` uses native BAM/FASTQ paths for supported remediation modes;
* dry-run and applied behavior remain visibly distinct;
* JSON contracts remain stable or are deliberately versioned.

Completion evidence:

* audited `src/forensics/deduplicate.rs` and confirmed FASTQ/FASTQ.GZ loading
  and output use the Milestone 4 `open_fastq_reader`,
  `read_next_fastq_record`, and `write_fastq_records` APIs;
* migrated BAM planning from the older `BamReader`/`read_next_record_layout`
  path to `BamScanner` plus `BamRecordView::to_record_layout`, preserving the
  existing native BAM writer bridge through header serialization,
  record-layout serialization, and `BgzfWriter`;
* strengthened `deduplicate` unit coverage for dry-run plans, applied FASTQ
  and BAM remediation, keep-first/keep-last behavior, whole-file append mode,
  local contiguous-block handling, reserved `global-exact` mode, removed-record
  report emission, checksum provenance, index invalidation reporting, and
  malformed FASTQ parse uncertainty;
* confirmed dry-run and applied payloads remain visibly distinct through
  execution, summary, output, checksum, and notes fields;
* extended `scanner_microbench --bamana-bin` command smoke timings to include
  a full-scan `deduplicate` dry-run plan and updated the benchmark result
  schema plus Sphinx benchmark documentation;
* kept the governed `deduplicate` JSON contract stable while adding hardening
  tests, native scanner migration, and benchmark smoke coverage.

### M7.7 Harden `forensic_inspect` Provenance Boundary

Status: complete.

Tasks:

* audit `forensic_inspect` native header and scanner-backed evidence paths;
* strengthen tests for header evidence, read-group evidence, program evidence,
  read-name regimes, aux-tag regimes, duplication hallmarks, bounded scans,
  full scans, and max-finding behavior where needed;
* ensure docs state that `forensic_inspect` is evidence-driven provenance
  inspection, not fraud detection or full validation;
* add command-level smoke timing where benchmark hooks support it.

Acceptance criteria:

* `forensic_inspect` uses native header/scanner evidence for supported BAM
  paths;
* findings remain caveated and evidence-scoped;
* JSON contracts remain stable or are deliberately versioned.

Completion evidence:

* audited `src/forensics/forensic_inspect.rs` and confirmed BAM header
  evidence comes from the native header payload while body evidence uses
  `BamScanner`, `BamRecordView`, native read-name access, native aux-tag
  traversal, and native sequence/quality decoding for duplication hallmarks;
* strengthened `forensic_inspect` unit coverage for read-group/header mismatch
  evidence, duplicate program-chain evidence, clean full-scan behavior,
  read-name regime shifts, aux-tag regime shifts, duplication hallmarks,
  bounded scan caveat reporting, full-scan evidence scopes,
  `max_findings` truncation, and non-BAM format rejection;
* confirmed findings remain evidence-scoped and caveated as provenance
  inspection rather than structural validation, duplicate marking, biological
  interpretation, or fraud attribution;
* extended `scanner_microbench --bamana-bin` command smoke timings to include
  a full-scan `forensic_inspect` path with explicit header, read-group,
  program, read-name, tag, and duplication scopes, and updated the benchmark
  result schema plus Sphinx benchmark documentation;
* kept the governed `forensic_inspect` JSON contract stable while adding
  hardening tests and benchmark smoke coverage.

### M7.8 Strengthen M7 Cross-Command Safety And Output Guarantees

Status: complete.

Tasks:

* audit output temp-file, overwrite, force, dry-run, checksum verification, and
  reindex behavior across `reheader`, `annotate_rg`, and `deduplicate`;
* ensure mutation commands never report applied changes before writes and
  verification steps complete;
* ensure forensic and inspection commands never write output artifacts unless
  explicitly requested;
* add focused tests for error cleanup and no-output-on-failure behavior where
  practical.

Acceptance criteria:

* mutation/remediation commands have consistent write-safety behavior;
* dry-run behavior is side-effect bounded;
* failures include useful path context and do not leave misleading success
  payloads.

Completion evidence:

* audited output temp-file, overwrite, `--force`, dry-run,
  checksum-verification, and reindex behavior across `reheader`,
  `annotate_rg`, and `deduplicate`;
* confirmed `reheader` and `annotate_rg` applied writes use temporary BAM
  outputs before rename and reject existing output paths without `--force`
  before creating temporary output files;
* moved `deduplicate --emit-removed-report` path validation ahead of primary
  output writing so report-path failures do not leave applied remediation
  outputs behind;
* added focused no-output-on-failure coverage for `reheader`, `annotate_rg`,
  and `deduplicate`, including sentinel preservation, temp-output absence, and
  failure payload checks;
* confirmed dry-run paths remain side-effect bounded for mutation/remediation
  outputs, with checksum and reindex work reported as deferred when applicable;
* updated M7 roadmap and Sphinx mutation-forensics documentation with the
  cross-command output-safety guarantees.

### M7.9 Strengthen M7 Dependency Boundaries And Benchmarks

Status: complete.

Tasks:

* extend dependency-boundary tests to name the M7 command set;
* ensure production direct `noodles` imports remain limited to documented CRAM
  compatibility paths;
* add or formalize command-level benchmark hooks for the M7 command set;
* record benchmark interpretation notes that distinguish command startup,
  scan cost, rewrite cost, compression cost, checksum verification, and
  dry-run behavior.

Acceptance criteria:

* dependency-boundary tests explicitly protect the M7 command set;
* every M7 command has runnable smoke benchmark or timing evidence;
* benchmark notes are documented and do not imply comparator parity where none
  exists.

Completion evidence:

* extended M7 dependency-boundary protection to include the scanner-backed
  `deduplicate` BAM planning path (`BamScanner`, `BamRecordView`, and native
  aux-tag traversal) alongside the existing writer bridge;
* added a contract test requiring the M7 oracle policy to name `reheader`,
  `annotate_rg`, `inspect_duplication`, `deduplicate`, and
  `forensic_inspect` as one mutation/forensics boundary;
* documented the M7 testing-oracle boundary, including native ownership of
  mutation safety, remediation policy, scanner evidence, FASTQ
  duplication/remediation expectations, and malformed-input failures;
* added contract coverage requiring M7 command timing rows in
  `header_microbench` and `scanner_microbench` result schemas and matching
  Sphinx benchmark documentation for every M7 command;
* updated header/scanner benchmark notes to distinguish process startup, scan
  cost, rewrite cost, compression cost, checksum verification cost, dry-run
  behavior, and the absence of comparator parity claims;
* updated the M7 roadmap and Sphinx mutation-forensics notes with the
  dependency-boundary and benchmark guardrails.

### M7.10 Close Milestone 7

Status: complete.

Tasks:

* run `cargo test`;
* run `cargo test --test contract`;
* run M7 command benchmark or smoke benchmark profiles;
* run the Sphinx documentation build;
* update `docs/roadmap/milestone-07-mutation-forensics.md`,
  `docs/roadmap/current_milestone.md`, README status text, Sphinx technical
  notes, and this task map with final Milestone 7 completion evidence;
* commit and push the closing milestone change.

Acceptance criteria:

* all M7.1 through M7.10 tasks are complete;
* `reheader`, `annotate_rg`, `inspect_duplication`, `deduplicate`, and
  `forensic_inspect` are documented with native mutation/remediation/forensics
  evidence;
* full tests, contract tests, Sphinx, and M7 command benchmark smoke checks
  pass;
* production `noodles` usage remains isolated to documented CRAM
  compatibility, tests, oracles, and fixtures;
* Milestone 7 completion evidence is recorded in the repository.

Completion evidence:

* ran `cargo test`;
* ran `cargo test --test contract`;
* ran `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html`;
* ran `cargo build --bin bamana --bin header_microbench --bin
  scanner_microbench`;
* ran M7 command smoke profiles:
  `header_microbench --profile small --iterations 1 --bamana-bin
  target/debug/bamana` reported `verify:1/1`, `header:1/1`,
  `reheader:1/1`, and `annotate_rg:1/1`, and `scanner_microbench --profile
  small --iterations 1 --bamana-bin target/debug/bamana` reported
  `summary:1/1`, `check_sort:1/1`, `check_map:1/1`, `validate:1/1`,
  `check_tag:1/1`, `subsample_bam:1/1`, `inspect_duplication:1/1`,
  `deduplicate:1/1`, and `forensic_inspect:1/1`;
* updated Milestone 7 roadmap, current milestone notes, README status text,
  Sphinx technical notes, and this task map with final Milestone 7 completion
  evidence;
* confirmed production direct `noodles` usage remains isolated to documented
  CRAM compatibility through contract tests;
* Milestone 7 is complete as of 2026-05-22, and Milestone 8 became active on
  2026-05-22 after that closeout was recorded.

## Milestone 8 Definition

Milestone 8 is complete only when Bamana's large transform, checksum, explode,
and ingest command wave is hardened on native substrates. The milestone covers
`sort`, `merge`, `explode`, `checksum`, and `consume`. It follows the mutation
and forensics wave because these commands have the largest operational blast
radius: they reorder records, combine files, split files, define verification
domains, or normalize heterogeneous inputs into BAM outputs.

## Milestone 8 Planned State

Status: complete as of 2026-05-23. Milestone 8 became active only after
Milestone 7 closed on 2026-05-22, and closed after M8.1 through M8.10
completed. The earlier command waves established stable native reader, writer,
checksum, dependency-boundary, and benchmark conventions.

Known present pieces:

* `sort` already rewrites BAM through `src/bam/sort.rs`, supports coordinate
  and queryname ordering, updates `@HD` sort metadata, writes BAM through the
  native BGZF writer, and can request canonical checksum verification;
* `merge` already combines multiple BAM inputs, applies conservative header
  compatibility checks, supports input-order and sorted output modes, writes
  BAM through the native BGZF writer, and can request canonical checksum
  verification;
* `checksum` already exposes deterministic header and record checksum domains,
  including order-sensitive and order-insensitive modes plus tag and primary
  record filters;
* `explode` already supports BAM, SAM, and FASTQ.GZ inputs, uses
  `FASTQ.GZI` planning for FASTQ.GZ shards, and writes contiguous output
  shards while preserving encounter order within each shard;
* `consume` already performs discovery, format classification, mixed-format
  policy enforcement, FASTQ/SAM/BAM normalization, explicit CRAM reference
  policy handling, threaded FASTQ.GZ import, and dry-run reporting;
* native BGZF, header, scanner, FASTQ, writer, and `FASTQ.GZI` substrates are
  planned to be complete before M8 activation.

Known gaps:

* `sort`, `merge`, `checksum`, `explode`, and BAM-side `consume` still use
  older `BamReader::open`, `parse_bam_header_from_reader`, and
  `read_next_record_layout` paths in several places rather than a
  scanner-owned raw-record or native writer bridge;
* in-memory first-slice strategies remain explicit but need M8 evidence around
  limits, user-facing caveats, and benchmark interpretation;
* `consume` checksum verification remains planned in the current slice and
  needs either implementation or precise M8 deferral language;
* `explode` shard planning needs a full M8 audit across BAM, SAM, and
  FASTQ.GZ, especially around checksums, index metadata, and exact guarantees;
* command contracts and examples need a fresh pass for transform safety,
  ordering semantics, checksum domains, shard boundaries, ingest policy, CRAM
  compatibility boundaries, and deferred index behavior;
* command-level benchmark smoke evidence is not yet recorded for the full M8
  command set;
* dependency-boundary tests do not yet name `sort`, `merge`, `explode`,
  `checksum`, and `consume` as one protected milestone set.

Milestone 8 closeout evidence must include:

* all M8.1 through M8.10 tasks complete;
* `sort` documented and tested as native BAM ordering and rewriting with
  explicit checksum/index caveats;
* `merge` documented and tested as native BAM merging with explicit header
  compatibility and ordering semantics;
* `checksum` documented and tested as deterministic native header and record
  checksum domains;
* `explode` documented and tested as native BAM/SAM/FASTQ.GZ sharding with
  explicit shard-boundary guarantees;
* `consume` documented and tested as native discovery, policy, and
  normalization with CRAM compatibility isolated;
* command JSON contracts remaining stable or deliberately versioned;
* command-level benchmark or smoke benchmark evidence for the M8 command set;
* production `noodles` usage remaining isolated to CRAM compatibility, tests,
  oracles, and fixtures.

Command-surface scope:

* Milestone 8 evidence is limited to large transform, checksum, explode, and
  ingest commands: `sort`, `merge`, `explode`, `checksum`, and `consume`.
* Native CRAM parsing, BAI/CSI index writing, broad external comparator parity,
  and external-memory sort/merge remain later work unless a specific M8 task
  explicitly includes them.

## Milestone 8 Task List

### M8.1 Activate Milestone 8 Scope And Baseline

Status: complete.

Tasks:

* update `docs/roadmap/current_milestone.md` so Milestone 8 is the active
  milestone only after Milestone 7 is complete;
* update roadmap/task-map status so Milestones 1 through 7 remain recorded
  with their correct completion state;
* audit `sort`, `merge`, `explode`, `checksum`, and `consume` command
  implementations, helper modules, tests, contracts, examples, and docs;
* record which command paths already use native BGZF, header, scanner, FASTQ,
  `FASTQ.GZI`, writer, and checksum primitives and which still need hardening;
* update README, CLI docs, Sphinx docs, and roadmap docs if the active
  milestone status changes visible project guidance.

Acceptance criteria:

* current milestone documentation names Milestone 8 as active only when M7 is
  closed;
* the task map records present M8 command evidence and gaps;
* no command behavior changes are made unless required by the baseline audit;
* public contract commands remain explicitly protected.

Completion evidence:

* confirmed `docs/roadmap/current_milestone.md`, README, roadmap overview, and
  Sphinx notes record Milestone 7 complete and Milestone 8 active only after
  the M7 closeout;
* audited `sort`, `merge`, `explode`, `checksum`, and `consume`
  implementations, command orchestration, tests, schemas, examples, fixtures,
  README coverage, CLI contracts, JSON-output docs, and Sphinx notes;
* recorded that `sort` and `merge` already use native transform engines,
  native header rewriting, native BGZF writing, and optional canonical
  checksum verification;
* recorded that `checksum` already exposes deterministic native header and
  record checksum domains with explicit mode/filter semantics;
* recorded that `explode` already supports BAM, SAM, and FASTQ.GZ sharding,
  with `FASTQ.GZI` metadata available for FASTQ.GZ shard planning;
* recorded that `consume` already performs discovery, classification,
  mixed-format policy enforcement, FASTQ/SAM/BAM normalization, explicit CRAM
  reference-policy handling, threaded FASTQ.GZ import, and dry-run reporting;
* recorded M8 hardening gaps around older `BamReader::open`,
  `parse_bam_header_from_reader`, and `read_next_record_layout` use,
  in-memory first-slice strategies, `consume --verify-checksum`,
  `explode` shard guarantees, dependency-boundary coverage, and command-level
  benchmark smoke hooks;
* kept command behavior unchanged while updating roadmap, Sphinx, and task-map
  baseline evidence.

### M8.2 Freeze Transform, Checksum, Explode, And Ingest Contracts

Status: complete.

Tasks:

* audit JSON schemas and success/failure examples for `sort`, `merge`,
  `explode`, `checksum`, and `consume`;
* confirm CLI documentation describes ordering semantics, checksum domains,
  shard guarantees, ingest policy, dry-run behavior, CRAM compatibility, and
  deferred index behavior accurately;
* add or update fixtures for sorted BAMs, merge compatibility, shard planning,
  checksum filters, FASTQ.GZ `FASTQ.GZI` planning, and mixed-format ingest
  where gaps exist;
* record any intentional contract changes before implementation hardening.

Acceptance criteria:

* M8 command schemas and examples exist and parse;
* contract tests fail if governed M8 command documentation, schemas, or
  examples disappear;
* transform, checksum, shard, and ingest claims are documented without
  overclaiming.

Completion evidence:

* audited JSON schemas and canonical success/failure examples for `sort`,
  `merge`, `explode`, `checksum`, and `consume`;
* added contract coverage requiring all five M8 commands to keep JSON schemas,
  canonical success/failure examples, CLI contracts, JSON-output docs,
  user-facing CLI docs, README coverage, and Sphinx transform/ingest notes;
* added JSON-output documentation for `checksum`, `sort`, `merge`, and
  `explode`, complementing the existing `consume` payload documentation;
* confirmed CLI documentation describes ordering semantics, checksum domains,
  shard boundaries, ingest policy, dry-run behavior, CRAM reference policy,
  deferred checksum verification, deferred index behavior, `FASTQ.GZI`
  planning, and in-memory first-slice caveats without overclaiming;
* confirmed fixture manifest coverage for sorted BAMs, merge compatibility,
  transform shard planning, checksum filters, FASTQ.GZ `FASTQ.GZI` planning,
  and mixed-format ingest scenarios;
* updated the M8 roadmap, Sphinx transform/ingest note, CLI docs, JSON-output
  docs, and this task map without changing command behavior.

### M8.3 Harden `sort` Native Ordering And Rewrite Boundary

Status: complete.

Tasks:

* audit `sort` use of native header parsing, record loading, ordering,
  serialization, BGZF writing, checksum verification, and index caveats;
* migrate older BAM reader/layout paths to scanner-owned raw-record or native
  writer bridges where behavior can be preserved;
* strengthen tests for coordinate order, queryname natural and lexicographical
  order, unmapped placement, reverse/tie ordering, header sort metadata,
  force/overwrite behavior, checksum verification, and index deferral where
  needed;
* ensure docs state memory and first-slice limitations honestly;
* add command-level smoke timing where benchmark hooks support it.

Acceptance criteria:

* `sort` uses native BAM primitives for supported ordering and writing paths;
* ordering semantics are deterministic and test-covered;
* checksum and index claims are only made where actually performed.

Completion evidence:

* audited `sort` native header parsing, record loading, ordering,
  serialization, BGZF writing, checksum verification, and index reporting;
* migrated `sort` record loading from the older `BamReader` /
  `read_next_record_layout` loop to `BamScanner` plus
  `BamRecordView::to_record_layout`, while preserving the existing native
  header rewrite and record-layout BGZF writer bridge;
* strengthened sort tests for coordinate ordering, unmapped placement,
  reverse/tie ordering, lexicographical queryname ordering, header sort
  metadata, overwrite safety, checksum verification reporting, index deferral,
  memory-limit caveats, and natural-queryname deferral;
* extended `scanner_microbench --bamana-bin` with a `sort` command smoke timing
  for coordinate rewrite with canonical checksum verification, and updated the
  benchmark result schema and Sphinx benchmark notes;
* updated M8 roadmap, Sphinx transform/ingest notes, and this task map with
  the scanner-backed sort boundary and first-slice limitations.

### M8.4 Harden `merge` Native Compatibility And Ordering Boundary

Status: complete.

Tasks:

* audit `merge` use of native header parsing, compatibility checks, record
  loading, ordering, serialization, BGZF writing, and checksum verification;
* migrate older BAM reader/layout paths to scanner-owned raw-record or native
  writer bridges where behavior can be preserved;
* strengthen tests for compatible headers, incompatible headers, input-order
  merge, coordinate merge, queryname merge, record counts, checksum
  verification, force/overwrite behavior, and index deferral where needed;
* ensure docs state that merge validity is limited to parsed and checked
  inputs;
* add command-level smoke timing where benchmark hooks support it.

Acceptance criteria:

* `merge` uses native BAM primitives for supported merge and writing paths;
* header compatibility and output ordering semantics are test-covered;
* JSON contracts remain stable or are deliberately versioned.

Completion evidence:

* audited `merge` record loading, header compatibility, ordering,
  serialization, BGZF writing, and checksum verification boundaries;
* migrated `merge` input loading from the older BAM reader/layout path to
  `BamScanner` plus `BamRecordView::to_record_layout`, while preserving the
  native record-layout writer bridge;
* strengthened merge engine and command tests for compatible headers,
  incompatible headers, input-order merge, coordinate merge, queryname merge,
  record counts, checksum verification, force/overwrite safety, and index
  deferral;
* documented that merge validity is limited to parsed headers, materialized
  records, compatibility checks, requested ordering, write completion, and
  optional checksum verification that actually ran;
* extended `scanner_microbench --bamana-bin` with a `merge` command smoke
  timing for coordinate merge with canonical checksum verification, and
  updated the benchmark result schema and Sphinx benchmark notes;
* updated M8 roadmap, Sphinx transform/ingest notes, and this task map with
  the scanner-backed merge boundary and first-slice limitations.

### M8.5 Harden `checksum` Native Domain Boundary

Status: complete.

Tasks:

* audit checksum modes, algorithms, header serialization domain, record
  serialization domain, canonical order-insensitive behavior, filters, and tag
  exclusions;
* migrate older BAM reader/layout paths to scanner-owned record or raw-record
  helpers where checksum semantics can be preserved;
* strengthen tests for raw record order, canonical record order, header-only,
  payload, all modes, mapped-only, primary-only, tag exclusion, duplicate
  multiplicity, and malformed inputs where needed;
* ensure docs state exactly what semantic equivalence each checksum mode does
  and does not imply;
* add command-level smoke timing where benchmark hooks support it.

Acceptance criteria:

* checksum domains are deterministic and documented;
* filters and excluded tags are part of the reported checksum definition;
* JSON contracts remain stable or are deliberately versioned.

Completion evidence:

* audited checksum modes, algorithm reporting, header serialization domain,
  record serialization domain, canonical order-insensitive behavior, filters,
  tag exclusions, and malformed auxiliary payload handling;
* migrated checksum record scanning from the older BAM reader/layout path to
  `BamScanner` plus `BamRecordView::to_record_layout`, while preserving the
  existing native checksum serialization domain;
* strengthened checksum engine and command tests for raw record order,
  canonical record order, header-only, payload, all modes, mapped-only,
  primary-only, tag exclusion, duplicate multiplicity, and malformed inputs;
* documented the exact semantic boundary for each checksum mode, including
  that canonical mode compares selected record content independent of record
  order while preserving multiplicity, and does not prove whole-file semantic
  equivalence outside the reported checksum definition;
* extended `scanner_microbench --bamana-bin` with a `checksum` command smoke
  timing for all-domain hashing with header inclusion, `NM` exclusion, and
  mapped-only filtering, and updated the benchmark result schema and Sphinx
  benchmark notes;
* updated M8 roadmap, Sphinx transform/ingest notes, and this task map with
  the scanner-backed checksum boundary and domain limitations.

### M8.6 Harden `explode` Native Sharding Boundary

Status: complete.

Tasks:

* audit BAM, SAM, and FASTQ.GZ `explode` paths, including header handling,
  record counting, shard planning, FASTQ.GZ `FASTQ.GZI` reuse/creation,
  compression, output naming, checksum metadata, and force behavior;
* migrate older BAM reader/layout paths to scanner-owned raw-record or native
  writer bridges where behavior can be preserved;
* strengthen tests for BAM shards, SAM shards, FASTQ.GZ shards, empty inputs,
  uneven shard sizes, too many shards, `FASTQ.GZI` planning, output collision,
  and encounter-order preservation where needed;
* ensure docs state shard boundary guarantees and limitations precisely;
* add command-level smoke timing where benchmark hooks support it.

Acceptance criteria:

* `explode` preserves encounter order within each shard for supported formats;
* FASTQ.GZ shard planning uses documented `FASTQ.GZI` behavior;
* JSON contracts remain stable or are deliberately versioned.

Completion evidence:

* audited BAM, SAM, and FASTQ.GZ `explode` paths, including header handling,
  record counting, shard planning, FASTQ.GZ `FASTQ.GZI` reuse/creation,
  compression, output naming, checksum metadata, and force behavior;
* migrated BAM explode counting and shard writing from the older BAM
  reader/layout path to `BamScanner` plus scanner-owned records bridged into
  Bamana's native BGZF writer;
* strengthened tests for BAM shards, SAM shards, FASTQ.GZ shards, empty BAM
  inputs, uneven FASTQ.GZ shard sizes, too many BAM shards, `FASTQ.GZI`
  planning evidence, output collision behavior, and encounter-order
  preservation;
* documented shard boundary guarantees and limitations: every supported record
  lands in exactly one reported range and encounter order is preserved within
  each shard, but reconstruction equivalence, uniform shard sizes, and generic
  random-access parallel gzip inflate are not implied;
* extended `scanner_microbench --bamana-bin` with an `explode` command smoke
  timing for scanner-backed BAM contiguous shard writing, and updated the
  benchmark result schema and Sphinx benchmark notes;
* updated README, JSON-output docs, CLI spec, M8 roadmap, Sphinx
  transform/ingest notes, and this task map with the scanner-backed explode
  boundary and shard limitations.

### M8.7 Harden `consume` Native Ingest And Policy Boundary

Status: complete.

Tasks:

* audit discovery, format probing, mixed-format policy, alignment versus
  unmapped modes, FASTQ/FASTQ.GZ import, SAM import, BAM pass-through,
  sorting, output writing, dry-run, thread handling, and CRAM reference policy;
* migrate older BAM reader/layout paths to scanner-owned raw-record or native
  writer bridges where behavior can be preserved;
* decide whether checksum verification is implemented in M8 or explicitly
  remains deferred with precise payload notes;
* strengthen tests for directories, recursive discovery, include/exclude
  filters if implemented, mixed-format rejection, FASTQ.GZ parallel import,
  indexed FASTQ.GZ import, BAM/SAM alignment mode, CRAM policy, dry-run, and
  force/overwrite behavior where needed;
* add command-level smoke timing where benchmark hooks support it.

Acceptance criteria:

* `consume` policy decisions are explicit and test-covered;
* native FASTQ/SAM/BAM ingest paths remain separate from CRAM compatibility;
* JSON contracts remain stable or are deliberately versioned.

Completion evidence:

* audited discovery, format probing, mixed-format policy, alignment and
  unmapped modes, FASTQ/FASTQ.GZ import, SAM import, BAM pass-through,
  sorting, output writing, dry-run, thread handling, and CRAM reference policy;
* migrated BAM alignment consume from the older BAM reader/layout path to
  `BamScanner` plus scanner-owned records bridged into Bamana's native BGZF
  writer;
* kept checksum verification explicitly deferred in M8.7 with a failure path
  that reports requested but unperformed checksum state before writing output;
* strengthened tests for recursive directory dry-run discovery, mixed-format
  rejection, FASTQ.GZ parallel import, indexed FASTQ.GZ import, BAM/SAM
  alignment mode, CRAM policy, dry-run behavior, checksum deferral, and
  force/overwrite behavior;
* extended `scanner_microbench --bamana-bin` with a `consume` command smoke
  timing for scanner-backed BAM alignment ingest, and updated the benchmark
  result schema and Sphinx benchmark notes;
* updated M8 roadmap, Sphinx transform/ingest notes, and this task map with
  the scanner-backed consume boundary and explicit deferred checksum/index
  behavior.

### M8.8 Strengthen M8 Cross-Command Output Safety

Status: complete as of 2026-05-23.

Tasks:

* audit temp-file, atomic rename, overwrite, force, dry-run, checksum
  verification, index creation, and cleanup behavior across `sort`, `merge`,
  `explode`, and `consume`;
* ensure commands never report written outputs before writes and verification
  steps complete;
* ensure failure paths include useful path context and avoid misleading success
  payloads;
* add focused tests for output collision, write failure, no-output-on-failure,
  and cleanup behavior where practical.

Acceptance criteria:

* transform and ingest commands have consistent write-safety behavior;
* dry-run behavior is side-effect bounded;
* checksum/index payloads match actual work performed.

Completion evidence:

* added a shared output finalization helper so `sort`, `merge`, `explode`, and
  `consume` publish completed temp files through a final rename step instead
  of deleting an existing output before finalization;
* kept existing collision checks and added helper coverage for no-force
  collision cleanup, force replacement, and multi-output preflight behavior;
* added a command-level `sort` finalization-failure regression proving a
  non-file output target remains untouched and the temp file is removed;
* documented the cross-command output-safety contract in README, CLI/spec,
  JSON-output docs, Sphinx notes, and the M8 roadmap.

### M8.9 Strengthen M8 Dependency Boundaries And Benchmarks

Status: complete as of 2026-05-23.

Tasks:

* extend dependency-boundary tests to name the M8 command set;
* ensure production direct `noodles` imports remain limited to documented CRAM
  compatibility paths;
* add or formalize command-level benchmark hooks for `sort`, `merge`,
  `explode`, `checksum`, and `consume`;
* record benchmark interpretation notes that distinguish command startup,
  full-record materialization, sorting cost, merge cost, compression cost,
  checksum domains, shard planning, ingest normalization, and CRAM
  compatibility behavior.

Acceptance criteria:

* dependency-boundary tests explicitly protect the M8 command set;
* every M8 command has runnable smoke benchmark or timing evidence;
* benchmark notes are documented and do not imply comparator parity where none
  exists.

Completion evidence:

* added an M8 dependency-boundary contract test that explicitly names `sort`,
  `merge`, `explode`, `checksum`, and `consume` and protects their production
  hot paths from direct `noodles` imports outside documented CRAM
  compatibility;
* updated the testing-oracle policy with an M8 transform and ingest boundary
  for native tests, permitted oracle roles, and the CRAM exception;
* formalized scanner microbenchmark interpretation notes for every M8 command
  hook, including startup, JSON emission, full-record materialization,
  in-memory sorting, merge compatibility, native BGZF compression, checksum
  domains, shard planning, ingest normalization, and CRAM compatibility
  behavior;
* added contract coverage requiring the M8 dependency and benchmark guardrails
  to stay documented.

### M8.10 Close Milestone 8

Status: complete as of 2026-05-23.

Tasks:

* run `cargo test`;
* run `cargo test --test contract`;
* run M8 command benchmark or smoke benchmark profiles;
* run the Sphinx documentation build;
* update `docs/roadmap/milestone-08-transform-ingest.md`,
  `docs/roadmap/current_milestone.md`, README status text, Sphinx technical
  notes, and this task map with final Milestone 8 completion evidence;
* commit and push the closing milestone change.

Acceptance criteria:

* all M8.1 through M8.10 tasks are complete;
* `sort`, `merge`, `explode`, `checksum`, and `consume` are documented with
  native transform/checksum/explode/ingest evidence;
* full tests, contract tests, Sphinx, and M8 command benchmark smoke checks
  pass;
* production `noodles` usage remains isolated to documented CRAM
  compatibility, tests, oracles, and fixtures;
* Milestone 8 completion evidence is recorded in the repository.

Completion evidence:

* ran `cargo test`;
* ran `cargo test --test contract`;
* ran `target/debug/scanner_microbench --profile small --iterations 1
  --bamana-bin target/debug/bamana --out /tmp/bamana-scanner-m8-10.json`,
  with all command timings reporting `1/1`;
* ran `python -m sphinx -b html docs/sphinx docs/sphinx/_build/html`;
* updated `docs/roadmap/milestone-08-transform-ingest.md`,
  `docs/roadmap/current_milestone.md`, README status text, Sphinx technical
  notes, and this task map with final Milestone 8 completion evidence;
* recorded Milestone 8 complete while leaving Milestone 9 planned for explicit
  activation by M9.1.

## Milestone 9 Definition

Milestone 9 is complete only when Bamana owns BAM index writing, deeper BAM
index validation, and the random-access groundwork needed by later indexed
region workflows. The milestone covers native BAI creation for supported BAM
inputs, `check_index` hardening, virtual-offset-backed reader/scanner
plumbing, scoped CSI decisions, and index-aware evidence paths in commands such
as `check_map` and `summary`.

## Milestone 9 Active State

Status: complete as of 2026-05-23. Milestone 9 became active through M9.1
after Milestone 8 closed, because transform and ingest commands had settled
output safety, sorting semantics, and checksum evidence enough for generated
BAM indices to become the next dependable contract. M9 closed after M9.1
through M9.10 completed.

Completed pieces:

* Milestone 1 introduced `VirtualOffset` groundwork for future BAI/CSI and
  random-access work;
* `src/bam/index.rs` already detects BAI, CSI, GZI, and unknown sidecar magic;
* `src/bam/index.rs` already discovers adjacent `.bam.bai`, `.bai`,
  `.bam.csi`, and `.csi` candidates with CSI preference support;
* `parse_bai` already performs BAI structural validation, reference-count
  reconciliation, bin/chunk/linear-index checks, metadata pseudo-bin summary
  extraction, and optional unplaced-unmapped count parsing;
* `parse_csi_header` already parses CSI header metadata enough to report
  detected-but-not-supported status;
* `check_index` already reports adjacent index presence, selected path, kind,
  BAI structural validity, CSI header status, staleness, compatibility, and
  apparent usability;
* `index` already creates native BAI sidecars for coordinate-sorted BAM inputs
  and `FASTQ.GZI` sidecars for FASTQ.GZ inputs;
* BAM `index` explicitly reports CSI writing as unimplemented.

Deferred beyond M9:

* BAM `index` cannot yet write real CSI output;
* public commands do not yet exercise random-access chunk traversal for
  acceleration or region filtering;
* CSI support remains header-only detection rather than scoped parsing/writing;
* index-aware `check_map` and `summary` evidence remains limited to validated
  BAI metadata and native scan fallback behavior.

Milestone 9 closeout evidence must include:

* all M9.1 through M9.10 tasks complete;
* BAM `index` documented and tested as creating real BAI for supported
  coordinate-sorted BAM inputs, or rejecting unsupported inputs precisely;
* `check_index` documented and tested as validating BAI/CSI sidecars to the
  implemented depth without overclaiming random-access proof;
* virtual-offset capture documented and tested as the substrate for BAI/CSI and
  later indexed region work;
* index-aware `check_map` and `summary` evidence documented as distinct from
  scan-derived evidence;
* CSI support documented as implemented to a scoped contract or explicitly
  deferred with precise reasons;
* command JSON contracts remaining stable or deliberately versioned;
* command-level benchmark or smoke benchmark evidence for the M9 command set;
* production `noodles` usage remaining isolated to CRAM compatibility, tests,
  oracles, and fixtures.

Command-surface scope:

* Milestone 9 evidence is limited to BAM index writing, BAM index inspection,
  virtual-offset random-access groundwork, and first index-aware command
  evidence in `check_map` and `summary`.
* Native CRAM parsing, indexed region selection commands, broad random-access
  APIs, external comparator parity, and FASTQ.GZI work beyond existing sidecar
  behavior remain later work unless a specific M9 task explicitly includes
  them.

## Milestone 9 Task List

### M9.1 Activate Milestone 9 Scope And Baseline

Status: complete as of 2026-05-23.

Tasks:

* update `docs/roadmap/current_milestone.md` so Milestone 9 is the active
  milestone only after Milestone 8 is complete;
* update roadmap/task-map status so Milestones 1 through 8 remain recorded
  with their correct completion state;
* audit `src/bam/index.rs`, `src/bgzf/virtual_offset.rs`,
  `src/bgzf/reader.rs`, `src/bam/scan.rs`, `src/commands/index.rs`,
  `src/commands/check_index.rs`, `check_map`, and `summary`;
* record which index and random-access pieces already exist and which still
  need implementation;
* update README, CLI docs, Sphinx docs, and roadmap docs if the active
  milestone status changes visible project guidance.

Acceptance criteria:

* current milestone documentation names Milestone 9 as active only when M8 is
  closed;
* the task map records present M9 index/random-access evidence and gaps;
* no command behavior changes are made unless required by the baseline audit;
* public contract commands remain explicitly protected.

Completion evidence:

* recorded Milestone 9 as active in README, CLI docs, roadmap docs,
  `docs/roadmap/current_milestone.md`, the M9 roadmap detail, Sphinx, and this
  task map after Milestone 8 closeout;
* audited `src/bam/index.rs`, `src/bgzf/virtual_offset.rs`,
  `src/bgzf/reader.rs`, `src/bam/scan.rs`, `src/commands/index.rs`,
  `src/commands/check_index.rs`, `check_map`, and `summary`;
* recorded present M9 evidence: virtual-offset type groundwork, index sidecar
  detection, BAI shallow metadata parsing, CSI header detection, check_index
  shallow validation/staleness reporting, FASTQ.GZI sidecar creation, BAM
  index honest deferral before M9.5, and index-versus-scan evidence
  distinctions;
* recorded M9 gaps at activation time: BAM BAI/CSI writing,
  scanner-exposed record virtual offsets before M9.3,
  BAI bin/chunk/linear-index construction, deeper
  `check_index` validation, scoped CSI decision, indexed `check_map`/`summary`
  acceleration, and M9 dependency/benchmark evidence;
* made no command behavior changes and left the public contract commands
  `benchmark`, `fastq`, and `unmap` protected.

### M9.2 Freeze Index Command Contracts And Fixtures

Status: complete as of 2026-05-23.

Tasks:

* audit JSON schemas and success/failure examples for `index` and
  `check_index`;
* confirm CLI documentation describes BAM BAI/CSI behavior, FASTQ.GZI behavior,
  overwrite rules, stale-index heuristics, and implemented validation depth
  accurately;
* add or update fixtures for valid BAI, invalid BAI, mismatched reference
  counts, stale indices, CSI headers, unsupported CSI, coordinate-sorted BAM,
  unsorted BAM rejection, and FASTQ.GZI sidecar behavior where gaps exist;
* record any intentional contract changes before index implementation work.

Acceptance criteria:

* `index` and `check_index` schemas and examples exist and parse;
* contract tests fail if governed index command documentation, schemas, or
  examples disappear;
* BAM index creation claims are not made until a sidecar is actually written.

Completion evidence:

* audited `spec/jsonschema/index.schema.json`,
  `spec/jsonschema/check_index.schema.json`,
  `spec/examples/index.success.json`, `spec/examples/index.failure.json`,
  `spec/examples/check_index.success.json`, and
  `spec/examples/check_index.failure.json`;
* confirmed governed docs described BAM BAI/CSI deferral before M9.5,
  FASTQ.GZI creation, overwrite behavior, timestamp-based stale-index
  heuristics, and current shallow validation depth;
* froze planned index fixtures for valid BAI, malformed BAI, mismatched BAI
  reference counts, stale BAI, CSI header detection, malformed CSI,
  coordinate-sorted BAM, unsorted BAM index rejection, FASTQ.GZ source, and
  FASTQ.GZI sidecar behavior;
* added contract coverage requiring the `index` and `check_index` docs,
  schemas, examples, and M9.2 fixture taxonomy to stay present;
* recorded no intentional command behavior changes before index implementation
  work; BAM index creation could not claim a created sidecar until native
  writing landed in later M9 tasks.

### M9.3 Expose BGZF Virtual Offsets During Native Scans

Status: complete as of 2026-05-23.

Tasks:

* extend native BGZF reader/scanner plumbing to expose compressed block offsets
  and uncompressed in-block offsets at record boundaries;
* ensure virtual offsets use the `VirtualOffset` type rather than ambiguous raw
  integers in new code;
* add tests for virtual offsets across single-block and multi-block BAM bodies;
* document how the offsets feed BAI/CSI chunk and linear-index construction.

Acceptance criteria:

* scanner/index code can obtain stable virtual offsets for alignment record
  starts and ends;
* virtual offsets remain bounded and packed according to BGZF semantics;
* tests cover block-boundary and multi-record cases.

Completion evidence:

* `NativeBgzfReader` now tracks compressed BGZF member starts and ends and
  reports the current cursor as a typed `VirtualOffset`;
* `BamReader` exposes virtual offsets for the native BGZF backend without
  assigning ambiguous raw integers to new scan/index code;
* `BamScanner` now exposes `next_record_with_virtual_offsets`, returning parsed
  records with typed start and end offsets while preserving existing
  `next_record` behavior for current command consumers;
* tests cover reader offsets across members, scanner offsets for multiple
  records inside one BGZF member, and scanner offsets when records begin in a
  later BGZF member and end at a member boundary;
* Sphinx and roadmap docs describe how the offsets feed future BAI/CSI chunk
  and linear-index construction.

### M9.4 Implement Native BAI Binning And Linear Index Construction

Status: complete as of 2026-05-23.

Tasks:

* implement BAI bin calculation for mapped BAM records;
* accumulate chunks per reference/bin using virtual-offset record spans;
* merge adjacent or overlapping chunks where the BAI format permits it;
* construct the BAI linear index from record start virtual offsets;
* account for unmapped and unplaced records according to the supported BAI
  contract.

Acceptance criteria:

* BAI index data structures can be built from scanner-owned record traversal;
* coordinate constraints and unsupported record shapes produce precise errors;
* unit tests cover representative bins, chunks, linear intervals, and unmapped
  accounting.

Completion evidence:

* added native in-memory BAI data structures for chunks, per-reference indexes,
  linear-index windows, mapped counts, reference-associated unmapped counts, and
  unplaced-unmapped counts;
* implemented `build_bai_index_from_bam` over
  `BamScanner::next_record_with_virtual_offsets`;
* implemented BAI bin calculation, reference-consuming CIGAR span calculation,
  chunk accumulation/coalescing, and 16kb linear-index construction;
* added precise failures for mapped records with negative reference ids,
  negative coordinates, reference ids outside the header dictionary,
  coordinates outside BAI's addressable range, and coordinate-order violations;
* added tests for representative bins, merged chunks, linear windows,
  reference-associated unmapped reads, unplaced unmapped reads, unsorted mapped
  records, and out-of-dictionary reference ids;
* documented that M9.4 built data structures only before M9.5 added BAI
  serialization, metadata pseudo-bin emission, atomic sidecar writing, and
  `bamana index` payload changes.

### M9.5 Implement BAM `index` BAI Writing

Status: complete.

Tasks:

* route BAM `index --format bai` and default BAM index creation through the
  native BAI builder and writer;
* enforce coordinate-sort or documented supported-order requirements;
* write BAI sidecars atomically with overwrite/force behavior consistent with
  other output commands;
* preserve FASTQ.GZI behavior for FASTQ.GZ inputs;
* update command payloads and examples only if the contract intentionally
  changes.

Acceptance criteria:

* BAM `index` can create a real BAI sidecar for supported BAM fixtures;
* unsupported sort order, invalid records, or impossible coordinates fail with
  structured errors;
* output paths, overwrite behavior, and `created` flags match actual work.

Completion evidence:

* implemented native BAI serialization in `src/bam/index.rs`, including
  regular bin chunks, metadata pseudo-bin mapped/unmapped counts, linear-index
  windows, and trailing unplaced-unmapped counts;
* routed BAM `index` default and `--format bai` output through native BAI
  building, temporary sidecar writing, and final rename semantics;
* preserved FASTQ.GZI command behavior and kept CSI output explicitly
  unimplemented;
* added regression coverage for generated BAI parsing, command success,
  header-declared unsupported sort order rejection, and existing BAM/GZI
  rejection;
* updated CLI contracts, JSON examples, roadmap notes, fixture planning, and
  Sphinx documentation for M9.5 behavior.

### M9.6 Harden `check_index` BAI And CSI Validation

Status: complete.

Tasks:

* validate BAI chunks, virtual-offset ordering, bin structure, linear-index
  shape, metadata pseudo-bin consistency, and reference counts where feasible;
* validate CSI headers and either implement scoped CSI structural parsing or
  preserve detected-but-not-supported behavior with precise notes;
* distinguish syntactic validity, staleness, compatibility, and random-access
  usability in payloads and docs;
* add tests for corrupt chunks, impossible virtual offsets, mismatched
  references, stale indices, unsupported CSI, and unknown index kinds.

Acceptance criteria:

* `check_index` reports validation depth precisely;
* invalid BAI/CSI sidecars fail deterministically with useful details;
* random-access usability is not overclaimed beyond implemented checks.

Completion evidence:

* hardened `parse_bai` to reject duplicate bins, impossible regular bin ids,
  zero regular chunks, backward chunk virtual offsets, unsorted chunks within a
  bin, backward non-zero linear-index offsets, malformed metadata pseudo-bins,
  mismatched reference counts, and trailing bytes;
* kept CSI in detected-but-not-supported mode while validating CSI header
  structure and reference-count agreement before reporting that status;
* updated `check_index` notes so BAI success reports implemented structural
  validation rather than shallow parsing;
* added regression tests for corrupt BAI chunks, impossible virtual-offset
  ordering, linear-index ordering, stale BAI usability, unsupported CSI,
  CSI reference mismatches, and unknown adjacent sidecars;
* updated CLI contracts, JSON examples, roadmap notes, and Sphinx
  documentation to distinguish syntactic validity, staleness, compatibility,
  and random-access usability without overclaiming random-access proof.

### M9.7 Add Random-Access Reader Groundwork

Status: complete.

Tasks:

* add a minimal native seek/read path that can reopen or reposition BGZF input
  at a `VirtualOffset` where the existing BGZF reader design allows it;
* add helpers to fetch records from BAI chunks for internal tests or first
  consumers;
* keep public region-query command behavior out of scope unless explicitly
  added later;
* add tests proving indexed chunks can retrieve expected records from supported
  fixtures.

Acceptance criteria:

* random-access helpers consume `VirtualOffset` values, not raw byte offsets;
* random-access tests prove at least one indexed chunk can be used to retrieve
  the expected records;
* unsupported random-access scenarios fail clearly.

Completion evidence:

* added `NativeBgzfReader::seek_virtual_offset`, which consumes typed
  `VirtualOffset` values, seeks to the compressed BGZF member, inflates it, and
  positions the in-block cursor;
* added native BAM reader and scanner seek plumbing for BGZF-backed inputs;
* added `BamScanner::raw_records_in_virtual_range` for internal consumers to
  retrieve positioned raw BAM records from typed virtual-offset ranges;
* kept public region-query command behavior out of scope;
* added tests for valid BGZF virtual-offset seeks, impossible in-block offsets,
  manual virtual-offset range traversal, native BAI chunk traversal, and
  empty-range rejection;
* updated roadmap, Sphinx, CLI, README, and taskmap documentation to describe
  the internal random-access substrate without claiming public command
  acceleration.

### M9.8 Integrate Index-Aware Evidence In First Consumers

Status: complete.

Tasks:

* audit `check_map` index-derived evidence against the deeper BAI validation
  and generated BAI sidecars;
* audit `summary` index-derived evidence against the deeper BAI validation and
  generated BAI sidecars;
* update payload notes and docs to distinguish generated-index evidence,
  discovered-index evidence, and scan-derived evidence;
* add tests for index-preferred and scan-fallback behavior after M9 index
  writing exists.

Acceptance criteria:

* `check_map` and `summary` use index evidence only when index validation says
  it is usable for the needed purpose;
* fallback scans remain native and documented;
* payloads clearly identify evidence source.

Completion evidence:

* centralized timestamp-based stale sidecar detection in `src/bam/index.rs`
  and reused it from `check_index`, `check_map`, and `summary`;
* `check_map` now uses BAI metadata only when the selected sidecar is not
  timestamp-stale, parses through the hardened BAI structural checks, and
  supplies complete per-reference mapped/unmapped metadata; otherwise it
  reports a native scanner fallback note;
* `summary` now applies the same usability gate before producing
  `index_derived` totals and records stale, malformed, unsupported, or
  incomplete sidecar fallback reasons in `semantic_note`;
* added command tests for generated Bamana BAI sidecars and timestamp-stale BAI
  fallback behavior in both `check_map` and `summary`;
* updated README, CLI, JSON-output, Sphinx, roadmap, and CLI contract
  documentation to distinguish generated/discovered BAI metadata evidence from
  native scan evidence;
* verification: `cargo test check_map`; `cargo test summary`;
  `cargo test --test contract`; `cargo test`;
  `sphinx-build -b html docs/sphinx docs/sphinx/_build/html`;
  `git diff --check`.

### M9.9 Strengthen M9 Dependency Boundaries And Benchmarks

Status: complete.

Tasks:

* extend dependency-boundary tests to name the M9 index and random-access
  command/substrate set;
* ensure production direct `noodles` imports remain limited to documented CRAM
  compatibility paths;
* add or formalize command-level benchmark hooks for BAM `index`,
  `check_index`, indexed `check_map`, and indexed `summary`;
* record benchmark interpretation notes that distinguish index construction,
  index validation, random-access lookup, scan fallback, and command startup
  costs.

Acceptance criteria:

* dependency-boundary tests explicitly protect the M9 command and substrate
  set;
* every M9 command path has runnable smoke benchmark or timing evidence;
* benchmark notes are documented and do not imply random-access or comparator
  parity where none exists.

Completion evidence:

* extended `tests/contract/dependency_boundary.rs` so the M9 native hot-path
  guard explicitly names `index`, `check_index`, indexed `check_map`, indexed
  `summary`, and the random-access substrate;
* added the Milestone 9 index/random-access oracle boundary to
  `docs/testing-oracles.md`;
* extended `scanner_microbench --bamana-bin` with `index_bam`, `check_index`,
  `check_map_indexed`, and `summary_indexed` command smoke timings;
* updated `scanner_microbench.schema.json`, benchmark result notes, Sphinx
  benchmark docs, README, current milestone docs, M9 roadmap docs, and M9
  Sphinx docs with interpretation notes distinguishing BAM index construction,
  BAI structural validation, index metadata-backed consumer evidence, scan
  fallback timings, random-access lookup deferral, process startup, JSON
  emission, and comparator non-parity;
* added contract coverage requiring the M9 dependency guardrails and benchmark
  hooks to stay documented and schema-governed;
* verification: `cargo test milestone_9 --test contract`;
  `cargo test m9_index_random_access_hot_paths_do_not_import_noodles --test
  contract`; `cargo build --bin bamana --bin scanner_microbench`;
  `target/debug/scanner_microbench --profile small --iterations 1 --bamana-bin
  target/debug/bamana --out /tmp/bamana-m9-9-scanner.json`, which reported
  `ok_count: 1` for `index_bam`, `check_index`, `check_map_indexed`, and
  `summary_indexed`; `cargo test --test contract`; `cargo test`;
  `sphinx-build -b html docs/sphinx docs/sphinx/_build/html`;
  `git diff --check`.

### M9.10 Close Milestone 9

Status: complete.

Tasks:

* run `cargo test`;
* run `cargo test --test contract`;
* run M9 command benchmark or smoke benchmark profiles;
* run the Sphinx documentation build;
* update `docs/roadmap/milestone-09-bam-index-random-access.md`,
  `docs/roadmap/current_milestone.md`, README status text, Sphinx technical
  notes, and this task map with final Milestone 9 completion evidence;
* commit and push the closing milestone change.

Acceptance criteria:

* all M9.1 through M9.10 tasks are complete;
* `index`, `check_index`, virtual-offset capture, and first index-aware
  consumers are documented with native BAM index/random-access evidence;
* full tests, contract tests, Sphinx, and M9 command benchmark smoke checks
  pass;
* production `noodles` usage remains isolated to documented CRAM
  compatibility, tests, oracles, and fixtures;
* Milestone 9 completion evidence is recorded in the repository.

Completion evidence:

* updated README, CLI docs, roadmap docs, Sphinx technical notes, and this task
  map to record Milestone 9 as complete as of 2026-05-23;
* recorded final M9 evidence for native BAI sidecar creation, hardened
  `check_index` validation, typed virtual-offset capture, first index-aware
  `check_map` and `summary` evidence, dependency-boundary guardrails, and M9
  benchmark smoke timings;
* preserved explicit deferrals for CSI writing, public indexed-region command
  acceleration, broad random-access APIs, and comparator parity beyond M9;
* verified production `noodles` usage remains isolated to documented CRAM
  compatibility, tests, oracles, and fixtures;
* closeout verification: `cargo test`; `cargo test --test contract`;
  `cargo build --bin bamana --bin scanner_microbench`;
  `target/debug/scanner_microbench --profile small --iterations 1 --bamana-bin
  target/debug/bamana --out /tmp/bamana-m9-10-scanner.json`;
  `sphinx-build -b html docs/sphinx docs/sphinx/_build/html`;
  `git diff --check`.

## Milestone 10 Definition

Milestone 10 is complete only when Bamana turns the Milestone 9 BAM index and
random-access substrate into bounded, user-visible indexed-region workflows.
The milestone covers region syntax and interval normalization, validated-index
chunk planning, random-access retrieval evidence, region-aware `check_map` and
`summary` behavior, and any first public indexed-region command or flag that is
explicitly promoted into the CLI contract.

## Milestone 10 Active State

Status: active as of 2026-05-23. Milestone 10 became active through M10.1 only
after Milestone 9 closeout recorded native BAI writing, deeper BAI validation,
typed virtual offsets, random-access helper limits, and index-aware
`check_map`/`summary` evidence.

Present baseline:

* Milestone 1 introduced `VirtualOffset` groundwork for later random-access
  paths;
* Milestone 3 introduced native BAM record scanning and borrowed record views;
* Milestone 6 established native `check_map` and `summary` inspection payloads;
* Milestone 9 completed native BAM index writing, deeper index validation, and
  minimal random-access reader helpers;
* `src/bam/index.rs` owns BAI parsing, implemented-depth BAI validation,
  CSI-header detection, `bai_bin_for_region`, BAI bin/chunk representation,
  and linear-index metadata needed for chunk planning;
* `src/bgzf/reader.rs` seeks by typed `VirtualOffset`, and `src/bam/scan.rs`
  exposes `raw_records_in_virtual_range` for internal random-access retrieval
  bounded by virtual offsets;
* `src/commands/check_map.rs` and `src/commands/summary.rs` already have
  index-aware evidence paths, but those paths are not region-scoped public
  workflows;
* the fixture plan already names valid coordinate BAM/BAI pairs, stale BAI,
  malformed BAI, mismatched-reference BAI, and CSI-header fixtures as important
  index-backed evidence.

Known gaps:

* M10.2 defines the internal region string grammar and interval normalization
  model, but no public command flag consumes it yet;
* region-file input remains explicitly deferred after M10.2;
* random-access chunk planning has not yet been promoted into command behavior;
* overlapping-region, duplicate-region, and multi-reference semantics are not
  specified;
* region-aware `check_map` and `summary` payloads do not yet distinguish
  requested-region scope from whole-file scope;
* no first public indexed-region command or flag has been frozen in CLI docs,
  JSON schemas, examples, or contract tests;
* benchmark evidence does not yet compare indexed region lookup with scan
  fallback behavior;
* CSI large-reference behavior and unsupported-index fallback policy remain
  dependent on the M9 CSI decision.

Milestone 10 closeout evidence must include:

* all M10.1 through M10.10 tasks complete;
* region syntax and optional region-file syntax documented as supported,
  rejected, or explicitly deferred;
* interval normalization tested for reference names, coordinate bases,
  inclusivity, empty intervals, unknown references, and out-of-range intervals;
* indexed chunk planning tested against validated BAI or scoped CSI fixtures;
* region-aware `check_map` and `summary` evidence documented as distinct from
  whole-file evidence;
* any new public command or flag covered by schemas, examples, Sphinx docs,
  CLI docs, and contract tests;
* indexed versus scan fallback benchmark or smoke benchmark evidence recorded;
* production `noodles` usage remaining isolated to CRAM compatibility, tests,
  oracles, and fixtures.

Command-surface scope:

* Milestone 10 evidence is limited to BAM indexed-region workflows above the
  native index and random-access substrate.
* Native CRAM parsing, CRAM indexed queries, broad external comparator parity,
  pileup/genotyping semantics, and biological interpretation remain later work
  unless a specific M10 task explicitly adds them.

## Milestone 10 Task List

### M10.1 Activate Milestone 10 Scope And Baseline

Status: complete.

Tasks:

* update `docs/roadmap/current_milestone.md` so Milestone 10 is the active
  milestone only after Milestone 9 is complete;
* update roadmap/task-map status so Milestones 1 through 9 remain recorded
  with their correct completion state;
* audit M9 outputs for BAI/CSI validation depth, virtual-offset capture,
  random-access helper limits, and index-aware consumer behavior;
* audit `src/commands/check_map.rs`, `src/commands/summary.rs`,
  `src/bam/index.rs`, `src/bam/scan.rs`, `src/bgzf/reader.rs`, and fixture
  plans for region-workflow readiness;
* update README, CLI docs, Sphinx docs, and roadmap docs if the active
  milestone status changes visible project guidance.

Acceptance criteria:

* current milestone documentation names Milestone 10 as active only when M9 is
  closed;
* the task map records present M10 indexed-region evidence and gaps;
* no command behavior changes are made unless required by the baseline audit;
* public contract commands, including `benchmark`, `fastq`, and `unmap`,
  remain explicitly protected.

Completion evidence:

* activated Milestone 10 only after Milestone 9 closeout was recorded as
  complete;
* updated README, CLI docs, roadmap summary, current milestone notes, the M10
  roadmap detail, Sphinx technical notes, and this task map to record M10 as
  active;
* audited and recorded present M10 substrate in `src/bam/index.rs`,
  `src/bgzf/reader.rs`, `src/bam/scan.rs`, `src/commands/check_map.rs`,
  `src/commands/summary.rs`, and fixture plans;
* recorded remaining M10 gaps for region syntax, region-file input, BAI chunk
  planning, overlapping and multi-reference semantics, region-aware payloads,
  indexed-versus-scan benchmarks, and CSI large-reference behavior;
* made no command behavior changes in the M10.1 baseline;
* kept public contract commands `benchmark`, `fastq`, and `unmap` explicitly
  protected.

### M10.2 Define Region Syntax And Interval Normalization

Status: complete.

Tasks:

* define the supported region string grammar, including reference names,
  coordinate base, interval inclusivity, whole-reference requests, and multiple
  region handling;
* define how reference names are resolved against the BAM header dictionary;
* reject empty, reversed, out-of-range, unknown-reference, and ambiguous
  intervals with structured errors;
* decide whether BED-like region files are in scope for M10 or explicitly
  deferred;
* document examples and non-goals before wiring region syntax into public
  command behavior.

Acceptance criteria:

* region parsing is deterministic and documented;
* interval normalization tests cover coordinate bases, inclusivity,
  whole-reference requests, unknown references, and invalid intervals;
* unsupported region-file behavior is either implemented to contract or
  rejected with a precise error.

Completion evidence:

* added `src/bam/region.rs` with native `reference` and
  `reference:start-end` parsing;
* normalized 1-based closed interval input to 0-based half-open coordinates
  while retaining original 1-based coordinates for reporting;
* resolved references against the BAM header dictionary and rejected duplicate
  names, unknown names, exact-reference-versus-interval ambiguity, empty
  strings, leading/trailing whitespace, zero coordinates, reversed intervals,
  non-numeric coordinates, out-of-range intervals, and zero-length
  whole-reference requests;
* preserved multiple requested regions in order without merging,
  deduplicating, or overlap resolution;
* explicitly deferred BED-like and line-oriented region files through
  `reject_region_file_request`;
* documented the M10.2 grammar and non-goals in README, CLI docs, roadmap
  docs, Sphinx notes, and this task map;
* added contract coverage for the M10.2 source and documentation baseline;
* made no public command behavior change and kept `benchmark`, `fastq`, and
  `unmap` protected.

### M10.3 Freeze Region Workflow Contracts And Fixtures

Status: complete.

Tasks:

* decide which command surfaces first expose indexed-region behavior, such as
  region-aware `check_map`, region-aware `summary`, or a new indexed selection
  command;
* update JSON schemas and examples for any command or flag promoted into the
  public contract;
* add or update fixtures for single-region, multi-region, overlapping-region,
  unknown-reference, empty-region, stale-index, missing-index, and unsupported
  index scenarios;
* update CLI docs, README command notes, Sphinx docs, and fixture plans before
  implementation claims are made.

Acceptance criteria:

* every promoted public region command or flag has schema, example, and docs
  coverage;
* fixture plans distinguish indexed success, scan fallback, and precise
  rejection scenarios;
* public contract tests fail if governed region workflow docs, schemas, or
  examples disappear.

Completion evidence:

* selected region-aware `check_map` and region-aware `summary` as the first
  planned public command surfaces, while deferring a standalone indexed
  selection command;
* froze the future repeated `--region <REGION>` CLI contract in
  `spec/cli/commands.md` without making the binary accept the flag yet;
* added `region_scope` schema definitions to `check_map` and `summary` JSON
  schemas with normalized region objects, coordinate model, duplicate policy,
  ordered region list, and `indexed`/`scan_fallback`/`rejected` execution
  outcomes;
* added canonical region success examples for `check_map` and `summary`;
* updated README, CLI docs, JSON-output docs, roadmap docs, Sphinx notes, and
  fixture plans with M10.3 region workflow contracts;
* updated fixture plans for single-region indexed success, multi-region
  indexed success, overlapping-region request-order behavior,
  unknown-reference rejection, empty-region rejection, stale-index fallback,
  missing-index fallback, and unsupported-index fallback;
* added contract coverage so region workflow docs, schemas, examples, and
  fixture-plan scenarios cannot disappear silently.

### M10.4 Implement Indexed Chunk Planning

Status: complete.

Tasks:

* convert normalized intervals into candidate BAI or scoped CSI bins;
* collect and coalesce candidate chunks using validated index data from M9;
* preserve enough provenance to explain which index and references informed
  each plan;
* reject unsupported index kinds, stale sidecars, incompatible references, and
  impossible virtual offsets before any region workflow claims indexed
  evidence;
* add unit tests for bin lookup, chunk coalescing, multi-reference planning,
  and no-hit intervals.

Acceptance criteria:

* chunk plans are deterministic for the same BAM, index, and requested regions;
* plans consume validated index summaries rather than shallow index detection;
* chunk planning errors are structured and useful.

Completion evidence:

* added `src/bam/region_plan.rs` as the internal indexed-region chunk planner;
* added `bai_bins_for_region` to expand normalized intervals across all BAI
  hierarchy levels;
* planned chunks from validated `BaiIndex` data rather than shallow sidecar
  discovery;
* preserved provenance for index path, index kind, reference names, reference
  indexes, requested regions, candidate bins, candidate chunks, and coalesced
  chunks;
* coalesced overlapping or adjacent virtual-offset chunks deterministically;
* rejected unsupported index kinds, stale BAI sidecars, reference-index
  incompatibility, empty region sets, and impossible virtual-offset chunks with
  structured errors;
* added unit tests for BAI bin lookup, chunk coalescing, multi-reference
  planning, no-hit intervals, and structured planning failures;
* documented M10.4 in README, CLI docs, roadmap docs, Sphinx notes, and this
  task map.

### M10.5 Implement Region-Bounded Random-Access Traversal

Status: complete.

Tasks:

* use M9 random-access helpers to traverse records from planned chunks;
* filter retrieved records against normalized intervals to account for broad
  bins and overlapping chunks;
* define and implement duplicate-record handling for overlapping regions;
* keep scan fallback native and explicit when no usable index is available;
* add tests that prove indexed traversal returns the expected records and does
  not double-count overlapping chunks unless the contract explicitly says so.

Acceptance criteria:

* indexed traversal returns records that overlap the requested intervals under
  the documented semantics;
* random-access traversal uses `VirtualOffset` values rather than raw byte
  offsets;
* fallback behavior is documented and visible in command payloads.

Completion evidence:

* added `src/bam/region_traversal.rs` with
  `traverse_planned_region_chunks` for planned `VirtualOffset` traversal
  through `raw_records_in_virtual_range`;
* filtered retrieved records against normalized intervals by reference and
  overlap so broad BAI bins do not leak records into region results;
* deduplicate records by virtual-offset range and merge matched region strings
  when overlapping chunks or overlapping region requests select the same
  record;
* represented missing or unusable index state as the explicit scan fallback
  `NativeScanRequired`;
* added traversal unit tests for region filtering, overlapping-region
  deduplication, and native fallback reporting;
* documented M10.5 in README, CLI docs, roadmap docs, Sphinx notes, and this
  task map.

### M10.6 Add Region-Aware `check_map`

Status: complete.

Tasks:

* add region request handling to `check_map` only after M10.2 through M10.5
  have stable substrate behavior;
* report requested regions, normalized intervals, evidence source, index path,
  fallback mode, and scan limits in the payload;
* ensure region-scoped totals cannot be confused with whole-file totals;
* add tests for indexed success, scan fallback, unknown reference, empty
  interval, overlapping intervals, and stale-index rejection or fallback.

Acceptance criteria:

* `check_map` can report mapping evidence for requested BAM regions when a
  usable index is present;
* region-scoped payload fields are explicit and do not reuse whole-file names
  ambiguously;
* unsupported region requests fail deterministically.

Completion evidence:

* added public `check_map --region <REGION>` CLI handling for repeated M10.2
  region strings;
* wired usable BAI sidecars through M10.4 chunk planning and M10.5
  random-access traversal;
* added native scan fallback for missing, stale, unsupported, or invalid index
  state with `fallback_mode: native_scan_required` and `scan_records_limit`;
* kept region-scoped counters separate as `region_records_examined`,
  `region_mapped_records_observed`, and
  `region_unmapped_records_observed`;
* added tests for indexed success, scan fallback, unknown reference, empty
  interval, overlapping intervals, and stale-index fallback;
* updated the `check_map` JSON schema, region example, CLI contract, README,
  CLI docs, JSON docs, roadmap docs, Sphinx notes, and this task map.

### M10.7 Add Region-Aware `summary`

Status: complete.

Tasks:

* add region request handling to `summary` only after M10.2 through M10.5 have
  stable substrate behavior;
* report requested regions, normalized intervals, evidence source, index path,
  fallback mode, and scan limits in the payload;
* decide which summary fields are meaningful for region scope and which remain
  whole-file only;
* add tests for indexed success, scan fallback, MAPQ/flag options, overlapping
  intervals, unknown reference, empty interval, and stale-index behavior.

Acceptance criteria:

* `summary` can report region-scoped operational evidence without implying
  whole-file validation;
* whole-file-only fields are omitted, marked as whole-file, or documented
  clearly;
* unsupported region requests fail deterministically.

Completion evidence:

* added public `summary --region <REGION>` CLI handling for repeated M10.2
  region strings;
* wired usable BAI sidecars through M10.4 chunk planning and M10.5
  random-access traversal;
* added native scan fallback for missing, stale, unsupported, or invalid index
  state with `fallback_mode: native_scan_required` and `scan_records_limit`;
* kept region-scoped operational `counts`, `fractions_observed`, `mapq`,
  `mapping`, `anomalies`, and optional `flag_categories` separate from
  whole-file totals;
* omitted whole-file BAI mapped/unmapped totals from region-scoped
  `index_derived` because the index is used only to find records;
* added tests for indexed success, scan fallback, MAPQ/flag options,
  overlapping intervals, unknown reference, empty interval, and stale-index
  fallback;
* updated the `summary` JSON schema, region example, CLI contract, README, CLI
  docs, JSON docs, roadmap docs, Sphinx notes, and this task map.

### M10.8 Decide And Implement First Indexed Selection Surface

Status: complete.

Tasks:

* decide whether M10 promotes a new public indexed region selection command,
  extends an existing command, or deliberately defers selection to a later
  milestone;
* if promoted, define output semantics, header preservation, record ordering,
  duplicate-region behavior, index invalidation or regeneration notes, and
  write-safety behavior;
* if deferred, document the precise reason and the substrate evidence that
  remains ready for later selection work;
* update schemas, examples, CLI docs, README, Sphinx docs, and fixtures for the
  chosen decision.

Acceptance criteria:

* the first indexed selection surface is either implemented to a public
  contract or explicitly deferred with a precise rationale;
* no command claims region selection support unless output behavior and
  write-safety semantics are documented and tested;
* public contract command coverage remains stable.

Completion evidence:

* deliberately deferred a public indexed region selection command for M10.8;
* documented that `check_map --region <REGION>` and
  `summary --region <REGION>` are read-only evidence surfaces and no command
  claims selected-record output;
* documented the required future contract for output semantics, header
  preservation, record ordering, duplicate-region behavior, index invalidation
  or regeneration notes, and output write-safety behavior;
* documented the ready substrate for future selection work: M10.2 region
  parsing, M10.4 BAI chunk planning, M10.5 region traversal, M10.6
  `check_map --region <REGION>`, and M10.7
  `summary --region <REGION>`;
* updated README, CLI docs, JSON docs, CLI contract, fixture planning, roadmap
  docs, Sphinx docs, and this task map;
* added a contract test that prevents selected-record output from being implied
  before a public command contract exists.

### M10.9 Strengthen M10 Dependency Boundaries And Benchmarks

Status: pending.

Tasks:

* extend dependency-boundary tests to name the M10 indexed-region command and
  substrate set;
* ensure production direct `noodles` imports remain limited to documented CRAM
  compatibility paths;
* add or formalize command-level benchmark hooks for indexed-region
  `check_map`, indexed-region `summary`, chunk planning, random-access
  traversal, and scan fallback;
* record benchmark interpretation notes that distinguish index lookup,
  random-access traversal, region filtering, scan fallback, and command startup
  costs.

Acceptance criteria:

* dependency-boundary tests explicitly protect the M10 command and substrate
  set;
* every promoted M10 command path has runnable smoke benchmark or timing
  evidence;
* benchmark notes do not imply broad comparator parity, native CRAM indexed
  queries, or biological interpretation.

Completion evidence:

* pending.

### M10.10 Close Milestone 10

Status: pending.

Tasks:

* run `cargo test`;
* run `cargo test --test contract`;
* run M10 command benchmark or smoke benchmark profiles;
* run the Sphinx documentation build;
* update `docs/roadmap/milestone-10-indexed-region-workflows.md`,
  `docs/roadmap/current_milestone.md`, README status text, Sphinx technical
  notes, and this task map with final Milestone 10 completion evidence;
* commit and push the closing milestone change.

Acceptance criteria:

* all M10.1 through M10.10 tasks are complete;
* region syntax, chunk planning, random-access traversal, and promoted
  region-aware command surfaces are documented with native BAM indexed-region
  evidence;
* full tests, contract tests, Sphinx, and M10 command benchmark smoke checks
  pass;
* production `noodles` usage remains isolated to documented CRAM
  compatibility, tests, oracles, and fixtures;
* Milestone 10 completion evidence is recorded in the repository.

Completion evidence:

* pending.
