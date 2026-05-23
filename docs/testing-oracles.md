# Testing And Oracle Policy

External parser crates such as `noodles` may still be useful in the Bamana test
strategy even after they are demoted from hot-path production roles.

## Valid Oracle Uses

Examples of acceptable oracle uses:

* compare Bamana-native header parsing against an independent parser
* compare BAM serialization behavior in small fixtures
* validate edge-case fixture behavior in differential tests
* cross-check compatibility of small synthetic examples

## Invalid Oracle Uses

Oracle usage must not become a hidden production dependency. In particular:

* tests must not justify moving `noodles` into hot paths
* convenience in tests must not drive production architecture
* compatibility checks must remain separate from the core execution engine

## Review Rule

If `noodles` or similar crates are added to tests:

* the test should state that the crate is used as an oracle or compatibility
  comparator
* the same crate should not quietly become a production-path requirement for
  the command being tested

## Native Header Oracle Boundary

`tests/header_oracle.rs` is the explicit test-only oracle surface for Milestone
2 header comparisons. It may use `noodles` to compare valid header fixtures or
to document compatibility differences, but production header and verify paths
must remain Bamana-native and free of direct `noodles` imports.

Malformed-header failure expectations must stay in native unit tests and must
not depend on an external parser. Differential tests may explain where another
parser is stricter or more permissive, but the expected Bamana behavior remains
owned by the native header codec.

## Native Scanner Oracle Boundary

Milestone 3 scanner expectations follow the same boundary.
Malformed-record failure expectations must stay in Bamana-native scanner and record-view tests;
they must not depend on `noodles` or another external parser to define the
expected error. Differential scanner checks may compare native scanner views
against Bamana's owned `RecordLayout` bridge or a clearly labelled test-only
oracle, but production scanner and migrated record hot paths must remain free of
direct `noodles` imports.

## Native FASTQ Oracle Boundary

Milestone 4 FASTQ expectations are native-first. Malformed FASTQ and FASTQ.GZ failure expectations
must stay in Bamana-native tests under `src/fastq` and must not depend on
`noodles`, `bio`, `needletail`, `seq_io`, `rust-htslib`, or another external
generic bioinformatics parser to define expected behavior.

Differential checks may be added later as a clearly labelled test-only oracle
surface, but production FASTQ parser, writer, command-consumer, and FASTQ.GZI paths must remain
Bamana-native. This includes `src/fastq`, FASTQ-facing command consumers,
unmapped FASTQ consume paths, duplication inspection, and deduplication.

## Milestone 5 Proof-Command Oracle Boundary

The Milestone 5 proof-command set is `verify`, `header`, and `subsample`.
The production proof-command paths for these commands must remain Bamana-native
and free of direct `noodles` imports.

Test-only oracle usage remains limited to explicit oracle surfaces:

* `tests/header_oracle.rs` may compare valid native BAM header behavior for
  `verify` and `header` against `noodles`;
* BAM-side `subsample` expectations are owned by native scanner and command
  tests using `BamScanner`, `BamRecordView`, and the scanner-owned raw-record
  bridge;
* FASTQ-side `subsample` expectations are owned by native FASTQ reader,
  record, and writer tests.

Malformed proof-command failure expectations must stay in native tests. Any
future differential test for these commands must be clearly labelled as an
oracle or compatibility comparison and must not define the production execution
engine.

## Milestone 7 Mutation And Forensics Oracle Boundary

The Milestone 7 mutation and forensics command set is `reheader`,
`annotate_rg`, `inspect_duplication`, `deduplicate`, and `forensic_inspect`.
The production mutation, remediation, and forensics paths for these commands
must remain Bamana-native and free of direct `noodles` imports.

Test-only oracle usage remains limited to explicit oracle or compatibility
surfaces:

* `reheader` and `annotate_rg` expectations are owned by native header,
  record-layout, aux-tag, checksum-domain, and BGZF writer tests;
* BAM-side `inspect_duplication`, `deduplicate`, and `forensic_inspect`
  expectations are owned by native scanner, `BamRecordView`, native aux-tag,
  and record-layout bridge tests;
* FASTQ-side `inspect_duplication` and `deduplicate` expectations are owned by
  native FASTQ reader and writer tests;
* BAM-side `deduplicate` must keep scanner-backed planning distinct from the
  native writer bridge used for retained output records.

Malformed mutation and forensics failure expectations must stay in native
tests. Any future differential test for these commands must be clearly labelled
as an oracle or compatibility comparison and must not define the production
execution engine, mutation safety model, or remediation policy.

## Milestone 8 Transform And Ingest Oracle Boundary

The Milestone 8 transform and ingest command set is `sort`, `merge`,
`explode`, `checksum`, and `consume`. The production transform, checksum,
sharding, and ingest paths for these commands must remain Bamana-native and
free of direct `noodles` imports, except for the documented CRAM compatibility
boundary in `src/ingest/cram.rs`.

Test-only oracle usage remains limited to explicit oracle or compatibility
surfaces:

* `sort`, `merge`, `explode`, and `checksum` expectations are owned by native
  scanner, header, record-layout, checksum-domain, and BGZF writer tests;
* BAM-side `consume` expectations are owned by `BamScanner`, native
  record-layout, and native BGZF writer tests;
* SAM, FASTQ, and FASTQ.GZ `consume` expectations are owned by native SAM and
  FASTQ readers, writers, and `FASTQ.GZI` planning tests;
* CRAM `consume` remains a compatibility exception governed by explicit
  reference policy and must not expand into BAM, BGZF, FASTQ, or transform hot
  paths.

Malformed transform and ingest failure expectations must stay in native tests.
Any future differential test for these commands must be clearly labelled as an
oracle or compatibility comparison and must not define the production execution
engine, checksum semantics, sharding policy, output-safety model, or ingest
normalization policy.
