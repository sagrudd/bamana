# Bamana Task Map

This file tracks explicit development tasks for Bamana native-core milestones.
Milestone 1 is the Native BGZF Core milestone described in
`docs/roadmap/milestone-01-bgzf.md`. Milestone 2 is the Native BAM Header Codec
milestone described in `docs/roadmap/milestone-02-bam-header.md`. Milestone 3
is the Native BAM Record Scanner milestone described in
`docs/roadmap/milestone-03-bam-record-scan.md`.

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

Status: active.

Known present pieces:

* Milestone 1 native BGZF reading is complete and can feed BAM payload bytes;
* Milestone 2 native BAM header parsing is complete and can position readers at
  the first alignment record;
* `src/bam/records.rs` contains the current central record bridge through
  `read_next_record_layout`, which performs bounded layout checks and
  materializes read name, CIGAR, sequence, quality, and aux sections;
* `LightAlignmentRecord` already exposes many field-only command inputs, but it
  is derived from `RecordLayout` and is not yet a true selective scanner view;
* `src/bam/tags.rs` contains bounded aux traversal and tag lookup over
  materialized aux bytes;
* `check_sort`, `check_map`, `summary`, `check_tag`, `validate`,
  `inspect_duplication`, `forensic_inspect`, and BAM-side `subsample` already
  perform record-facing work through existing first-slice readers and helpers;
* dependency-boundary checks already prohibit production `noodles` usage
  outside the CRAM compatibility exception.

Known gaps:

* no dedicated `src/bam/scan.rs` selective scanner API is in place yet;
* no stable lightweight record-view contract owns core fields and aux
  boundaries;
* skip-oriented selective field extraction is not centralized;
* record-scanning consumers generally use the transitional `BamReader::open`
  gzip backend today rather than requiring the native BGZF backend;
* command consumers are not migrated onto a shared scanner substrate;
* scanner oracle coverage and malformed-record tests are not yet isolated;
* scanner microbenchmarks are not yet present;
* M3.2 through M3.10 remain outstanding.

Milestone 3 closeout evidence must include:

* all M3.1 through M3.10 tasks complete;
* `cargo test` passing;
* `cargo test --test contract` passing;
* Sphinx documentation building successfully;
* scanner microbenchmarks runnable with machine-readable output;
* selected first command consumers documented as using the native scanner;
* roadmap and task-map status updated to record Milestone 3 completion.

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

Status: pending.

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

* pending.

### M3.3 Implement Native Scan Loop

Status: pending.

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

* pending.

### M3.4 Add Selective Field Extraction Helpers

Status: pending.

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

* pending.

### M3.5 Add Aux Region Traversal Without Full Decode

Status: pending.

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

* pending.

### M3.6 Migrate `check_sort` To The Native Scanner

Status: pending.

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

* pending.

### M3.7 Migrate `check_map`, `summary`, And `check_tag` Scanner Consumers

Status: pending.

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

* pending.

### M3.8 Migrate Validation And Forensics First-Slice Consumers

Status: pending.

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

* pending.

### M3.9 Add Scanner Oracle, Dependency Boundary, And Microbenchmarks

Status: pending.

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

* pending.

### M3.10 Close Milestone 3

Status: pending.

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

* pending.
