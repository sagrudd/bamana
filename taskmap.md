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
`docs/roadmap/milestone-07-mutation-forensics.md`.

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

Status: active.

Known present pieces:

* `src/fastq/mod.rs` already contains Bamana-native FASTQ functionality,
  including `FastqRecord`, plain/gzip reader opening, record parsing, record
  counting, unmapped-BAM conversion, threaded FASTQ.GZ-to-BAM conversion,
  plain/gzip FASTQ writing, and selected HTS-style methylation header tag
  conversion;
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

* FASTQ code is still concentrated in `src/fastq/mod.rs` rather than split
  into explicit `reader`, `writer`, `record`, and gzip-oriented modules;
* the current `FastqRecord` owns all strings, so field-only consumers still
  allocate complete record lines;
* gzip behavior is extension-driven and uses generic gzip decoding, while the
  Milestone 4 boundary needs explicit FASTQ.GZ stream semantics and tests for
  multi-member and malformed gzip inputs;
* writer behavior exists but needs a clearer contract for line endings,
  finishing/flushing, gzip output, and round-trip validation;
* command consumers share helpers but have not been audited against a stable
  Milestone 4 reader/writer API;
* FASTQ microbenchmark hooks are present in the broader benchmark framework,
  but a small native parser/writer smoke benchmark and closeout evidence are
  not yet recorded in the milestone task map.

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

Status: pending.

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

* pending.

### M4.2 Split FASTQ Core Into Explicit Native Modules

Status: pending.

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

* pending.

### M4.3 Define Native FASTQ Record View And Owned Record Contract

Status: pending.

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

* pending.

### M4.4 Strengthen Plain FASTQ Reader Validation

Status: pending.

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

* pending.

### M4.5 Strengthen FASTQ.GZ Reader And Gzip Stream Semantics

Status: pending.

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

* pending.

### M4.6 Complete FASTQ Writer And Round-Trip Guarantees

Status: pending.

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

* pending.

### M4.7 Migrate FASTQ-Side `subsample` And `enumerate` Consumers

Status: pending.

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

* pending.

### M4.8 Migrate `consume`, Duplication, Deduplication, And Shard Consumers

Status: pending.

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

* pending.

### M4.9 Add FASTQ Oracle, Dependency Boundary, And Microbenchmarks

Status: pending.

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

* pending.

### M4.10 Close Milestone 4

Status: pending.

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

* pending.

## Milestone 5 Definition

Milestone 5 is complete only when the first proof commands have explicit
native-substrate evidence and no production `noodles` hot-path dependency.
The milestone covers `verify`, `header`, and `subsample` in that order.
`verify` and `header` are the proof commands for Milestone 1 BGZF plus
Milestone 2 header ownership. `subsample` is the first stronger end-to-end
proof across native BAM scanning, native FASTQ parsing, deterministic/random
selection, and native serialization or pass-through writing.

## Milestone 5 Planned State

Status: planned. Milestone 5 should not become active until Milestone 4 has
closed, because full `subsample` coverage depends on the stable native
FASTQ/FASTQ.GZ parser and writer APIs from Milestone 4.

Known present pieces:

* Milestone 1 native BGZF substrate is complete;
* Milestone 2 native BAM header codec is complete;
* Milestone 3 native BAM record scanner is complete for selected
  scanner-compatible consumers;
* Milestone 4 is active and is expected to stabilize the FASTQ parser/writer
  APIs needed by FASTQ-side `subsample`;
* production `verify` already routes through native BGZF probing plus
  `parse_bam_header_from_native_bgzf`;
* production `header` already routes through native BGZF probing plus
  `parse_bam_header_from_native_bgzf`;
* contract dependency-boundary tests already protect production native header,
  verify, scanner, and migrated hot paths from direct `noodles` imports;
* production `subsample` already has governed JSON contracts for BAM, FASTQ,
  and FASTQ.GZ inputs and explicit deterministic/random selection policies.

Known gaps:

* BAM-side `subsample` still uses `BamReader::open` and
  `read_next_record_layout` rather than the native scanner or a scanner-owned
  raw-record bridge;
* FASTQ-side `subsample` still depends on the current monolithic FASTQ helpers
  rather than the stable Milestone 4 reader/writer API;
* command-level benchmark deltas for `verify`, `header`, and `subsample` are
  not yet recorded as M5 evidence;
* dependency-boundary tests do not yet name the full M5 proof-command set as a
  single governed migration boundary;
* post-migration differential and fixture evidence for `subsample` across BAM,
  FASTQ, and FASTQ.GZ remains to be collected.

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

Status: pending.

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

* pending.

### M5.2 Freeze Proof-Command Contracts And Fixtures

Status: pending.

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

* pending.

### M5.3 Confirm And Harden `verify` Native Migration

Status: pending.

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

* pending.

### M5.4 Confirm And Harden `header` Native Migration

Status: pending.

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

* pending.

### M5.5 Migrate BAM-Side `subsample` To Native Scanner Or Raw-Record Bridge

Status: pending.

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

* pending.

### M5.6 Migrate FASTQ-Side `subsample` To Milestone 4 APIs

Status: pending.

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

* pending.

### M5.7 Strengthen M5 Dependency Boundaries

Status: pending.

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

* pending.

### M5.8 Add Differential And Fixture Coverage For `subsample`

Status: pending.

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

* pending.

### M5.9 Add Proof-Command Benchmark Evidence

Status: pending.

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

* pending.

### M5.10 Close Milestone 5

Status: pending.

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

* pending.

## Milestone 6 Definition

Milestone 6 is complete only when Bamana's first operational BAM inspection
and validation command wave is hardened on native substrates. The milestone
covers `check_eof`, `check_sort`, `check_map`, `summary`, `check_tag`, and
`validate`. It is the follow-on to Milestone 5: after proof commands establish
the native path, M6 makes the inspection commands dependable, well bounded,
benchmarked, and protected from production `noodles` hot-path regressions.

## Milestone 6 Planned State

Status: planned. Milestone 6 should not become active until Milestone 5 has
closed, because the proof-command migration should establish the final
dependency-boundary and benchmark conventions for this command wave.

Known present pieces:

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
  the scanner substrate and selected migrated hot paths.

Known gaps:

* the inspection command set has not yet been audited as one governed M6 wave;
* command contracts and examples need a fresh pass for bounded versus full-scan
  claims, index-derived versus scan-derived evidence, and validation caveats;
* `validate` needs explicit M6 evidence that it remains structural and does
  not overclaim biological, reference-level, or optional-field semantic
  correctness;
* command-level benchmark smoke evidence is not yet recorded for the full M6
  inspection wave;
* dependency-boundary tests do not yet name `check_eof`, `check_sort`,
  `check_map`, `summary`, `check_tag`, and `validate` as one protected
  milestone set.

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

Status: pending.

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

* pending.

### M6.2 Freeze Inspection Command Contracts And Examples

Status: pending.

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

* pending.

### M6.3 Harden `check_eof` Native BGZF Boundary

Status: pending.

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

* pending.

### M6.4 Harden `check_sort` Scanner Evidence

Status: pending.

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

* pending.

### M6.5 Harden `check_map` Index And Scanner Evidence

Status: pending.

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

* pending.

### M6.6 Harden `summary` Scanner Evidence

Status: pending.

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

* pending.

### M6.7 Harden `check_tag` Aux Traversal Evidence

Status: pending.

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

* pending.

### M6.8 Harden `validate` Structural Validation Boundary

Status: pending.

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

* pending.

### M6.9 Strengthen M6 Dependency Boundaries And Benchmarks

Status: pending.

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

* pending.

### M6.10 Close Milestone 6

Status: pending.

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

* pending.

## Milestone 7 Definition

Milestone 7 is complete only when Bamana's native mutation, conservative
remediation, and provenance-inspection command wave is hardened on native
substrates. The milestone covers `reheader`, `annotate_rg`,
`inspect_duplication`, `deduplicate`, and `forensic_inspect`. It follows the
inspection/validation wave because these commands have higher blast radius:
they either mutate output, remediate duplicated collection blocks, or assemble
operational provenance evidence.

## Milestone 7 Planned State

Status: planned. Milestone 7 should not become active until Milestone 6 has
closed, because the inspection/validation wave should establish the command
contract, benchmark, and dependency-boundary discipline this mutation and
forensics wave depends on.

Known present pieces:

* Milestone 2 native BAM header codec provides the substrate for `reheader`,
  `annotate_rg`, checksum-domain header serialization, and header evidence in
  forensic reports;
* Milestone 3 native BAM scanner already backs BAM-body scans in
  `inspect_duplication` and `forensic_inspect`;
* Milestone 4 is expected to stabilize native FASTQ/FASTQ.GZ parser and writer
  APIs used by FASTQ-side duplication and deduplication paths;
* `reheader` command orchestration already separates header-only mutation from
  record-level `RG:Z` annotation and exposes dry-run, rewrite, reindex, and
  checksum verification controls;
* `annotate_rg` command orchestration already enforces explicit record-mode and
  header-policy choices;
* `inspect_duplication` already supports BAM, FASTQ, and FASTQ.GZ inputs and
  uses scanner/native FASTQ helpers for current scan paths;
* `deduplicate` already implements conservative remediation modes for BAM,
  FASTQ, and FASTQ.GZ inputs, with dry-run and applied modes;
* `forensic_inspect` already uses scanner-backed BAM body evidence for
  read-group, read-name, aux-tag, and duplication-hallmark checks.

Known gaps:

* the M7 command set has not yet been audited as one governed mutation,
  remediation, and forensics wave;
* `deduplicate` BAM paths still use `BamReader::open`,
  `parse_bam_header_from_reader`, and `read_next_record_layout` rather than a
  scanner-owned raw-record or native writer bridge;
* `reheader` and `annotate_rg` implementation modules need a fresh native-core
  audit for header serialization, record serialization, checksum verification,
  and index invalidation boundaries;
* FASTQ-side duplication and deduplication paths need to be reconciled with the
  stable Milestone 4 reader/writer APIs;
* command contracts and examples need a fresh pass for mutation safety,
  dry-run/apply distinctions, remediation limits, and forensic caveats;
* command-level benchmark smoke evidence is not yet recorded for the full M7
  command set;
* dependency-boundary tests do not yet name `reheader`, `annotate_rg`,
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

Status: pending.

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

* pending.

### M7.2 Freeze Mutation And Forensics Contracts And Examples

Status: pending.

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

* pending.

### M7.3 Harden `reheader` Native Header Mutation Boundary

Status: pending.

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

* pending.

### M7.4 Harden `annotate_rg` Native Record Annotation Boundary

Status: pending.

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

* pending.

### M7.5 Harden `inspect_duplication` Native Scan Boundary

Status: pending.

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

* pending.

### M7.6 Harden `deduplicate` Conservative Remediation Boundary

Status: pending.

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

* pending.

### M7.7 Harden `forensic_inspect` Provenance Boundary

Status: pending.

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

* pending.

### M7.8 Strengthen M7 Cross-Command Safety And Output Guarantees

Status: pending.

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

* pending.

### M7.9 Strengthen M7 Dependency Boundaries And Benchmarks

Status: pending.

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

* pending.

### M7.10 Close Milestone 7

Status: pending.

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

* pending.
