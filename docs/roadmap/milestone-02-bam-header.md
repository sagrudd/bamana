# Milestone 2: Native BAM Header Codec

## Technical Goal

Implement Bamana-native BAM header parsing and serialization for:

* BAM magic
* `l_text`
* textual SAM-style header content
* binary reference dictionary
* deterministic reserialization

## Owned Modules

Primary ownership:

* `src/bam/header.rs`
* `src/bam/reader.rs`
* header serialization portion of `src/bam/write.rs`

Future extension targets:

* header mutation helpers for `reheader`
* compatibility-aware header merge helpers for `merge`

## Native Data Model Contract

The Milestone 2 header model preserves two related views of the BAM header:

* `raw_header_text` stores the SAM-style text exactly as declared by the BAM
  header prefix;
* `references` stores the binary reference dictionary in encounter order.

The binary reference dictionary is authoritative for BAM decoding. Each
reference record retains the binary name, non-negative length, and encounter
order index. Parsed textual `@SQ` metadata is retained as diagnostic and
user-facing metadata, including a `text_header_length` value when the textual
`LN` disagrees with the binary dictionary.

Reference reconciliation is non-destructive. Mismatches between textual `@SQ`
records and the binary dictionary are reported as warning diagnostics rather
than silently rewriting either view. Diagnostics cover missing, extra,
duplicate, reordered, name-mismatched, length-mismatched, and malformed textual
`@SQ` records. The binary dictionary remains authoritative for downstream BAM
decoding.

Unknown SAM-style header records remain representable through their raw line so
later commands can avoid lossy parsing. Writer consumers must serialize through
the native header module rather than casting or assembling BAM header bytes
themselves; values that do not fit BAM signed 32-bit fields must fail before
bytes are emitted.

## Deterministic Serialization Rules

BAM header payload serialization is deterministic:

* the emitted payload starts with the BAM magic bytes;
* `l_text` is the byte length of the selected SAM-style header text;
* SAM-style header text is emitted exactly as supplied by the native header
  view or command-specific header mutation helper;
* `n_ref` is the binary reference count;
* binary references are emitted in encounter-order index order;
* every binary reference name is emitted with exactly one required NUL
  terminator;
* every length and count is checked before serialization to ensure it fits the
  BAM signed 32-bit fields.

Checksum-domain header serialization is also centralized in the native header
module so checksum code does not duplicate the header representation rules.

## Dependencies / Prerequisites

Depends on:

* Milestone 1 native BGZF substrate

## Commands Enabled Or Migrated

Primary migration targets:

* `verify`
* `header`

Secondary beneficiaries:

* `reheader`
* `check_sort`
* `check_map`
* `summary`

## Remaining `noodles` Surface

Allowed after this milestone:

* CRAM compatibility
* tests and oracles for header differential checks, isolated to test-only
  surfaces such as `tests/header_oracle.rs`

Disallowed:

* production `header` or `verify` behavior backed by `noodles`

## Acceptance Criteria

* BAM header text and references parse natively
* malformed and negative lengths are detected safely
* BAM header serialization is deterministic
* textual and binary reference information are merged consistently
* production `header` command uses native BGZF streaming plus native BAM header
  parsing and does not rely on `noodles`
* production `verify` uses native BGZF plus native header path only and keeps
  alignment-record and EOF-marker checks out of scope

## Benchmark Hooks

* header parse latency microbenchmark
* header serialization microbenchmark
* startup / read-prefix cost comparison before and after migration
* command-level `verify` and `header` reruns in the benchmark framework where
  appropriate

## Risks / Follow-Up

* reserialization rules must remain stable for checksum and reheader work
* header mutation ergonomics should not introduce expensive generic structures
