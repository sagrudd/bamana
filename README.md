# Bamana

Bamana is a high-performance Rust toolkit for verification, quality control,
inspection, and transformation of BAM files and related bioinformatics formats.

The current repository contains the first concrete CLI slice for:

* `bamana identify <path>`
* `bamana verify --bam <bamfile>`
* `bamana check_eof --bam <bamfile>`
* `bamana header --bam <bamfile>`
* `bamana check_map --bam <bamfile>`
* `bamana check_sort --bam <bamfile>`
* `bamana check_index --bam <bamfile>`
* `bamana index --bam <bamfile>`
* `bamana summary --bam <bamfile>`
* `bamana check_tag --tag <TAG> --bam <bamfile>`
* `bamana validate --bam <bamfile>`
* `bamana checksum --bam <bamfile>`
* `bamana sort --bam <bamfile> --out <result.bam>`
* `bamana merge --bam <bamfile1> <bamfile2> ... --out <result.bam>`

All command output is JSON.

The current semantics are intentionally narrow:

* `identify` determines the most likely file type quickly using extension hints, magic bytes, and shallow text heuristics
* `verify` performs shallow BAM verification only by confirming a BAM-like BGZF container and `BAM\1` magic in the first inflated block
* `check_eof` checks only for the canonical 28-byte BGZF EOF marker
* `header` parses the BAM header only, including the binary reference dictionary and textual SAM-style header records
* `check_map` prefers index-derived mapping summaries when a usable BAI is present and otherwise falls back to scan-based evidence
* `check_sort` combines BAM header declarations with a bounded scan of alignment records to assess coordinate or queryname ordering
* `check_index` inspects adjacent BAM indices for presence, type, shallow syntactic validity, timestamp-based staleness, and apparent usability
* `index` currently validates the BAM and resolves the output index path and format, but reports index creation as not yet implemented in this slice
* `summary` provides a fast operational BAM overview from header metadata, optional index-derived totals, and bounded or full record scans
* `check_tag` tests for BAM auxiliary tag presence using a bounded scan by default and full-file absence only when a complete scan succeeds
* `validate` performs a deeper streaming BAM structural and internal-consistency pass than `verify`, with finding severities and bounded modes
* `checksum` computes explicit machine-verifiable checksum domains over deterministic BAM header and record serializations, with order-sensitive and order-insensitive modes
* `sort` rewrites a BAM into an explicitly requested order using a deterministic in-memory first-slice engine with optional canonical checksum verification
* `merge` combines multiple BAM inputs into one BAM using conservative header compatibility checks, explicit input-order or sorted output modes, and optional canonical checksum verification

Neither `verify` nor `check_eof` implies deep validation of the BAM payload.
`header` does not imply that alignment records are readable, that EOF is present, or that the full BAM body is valid.
`check_map` does not imply full BAM validity, EOF completeness, or validation of every alignment record.
`check_sort` does not imply full BAM validity, EOF completeness, or validation of every alignment record.
`check_index` does not imply that every random-access offset is correct or that the BAM and index are semantically matched beyond shallow inspection.
`index` does not imply that BAM index writing has completed unless the response explicitly reports a created output.
`summary` does not imply full BAM validity, valid EOF state, or validation of every optional field, tag, or record invariant.
`check_tag` does not imply full BAM validity, valid EOF state, or semantic correctness of tag values beyond the auxiliary-field traversal actually performed.
`validate` does not imply biological correctness, external reference concordance, or correctness of every optional-field semantic beyond the checks actually implemented.
`checksum` does not imply full BAM validity, biological correctness, or semantic equivalence under any mode other than the one explicitly reported in the response.
`sort` does not imply full BAM validity beyond what was parsed, semantic preservation unless checksum verification was actually performed, or index correctness unless index creation and inspection explicitly succeeded.
`merge` does not imply full validity of all inputs beyond what was parsed, semantic preservation unless checksum verification was actually performed, or index correctness unless index creation and inspection explicitly succeeded.

## Example Invocations

```bash
cargo run -- identify example.bam
cargo run -- verify --bam example.bam
cargo run -- check_eof --bam example.bam
cargo run -- header --bam example.bam
cargo run -- check_map --bam example.bam
cargo run -- check_map --bam example.bam --sample-records 50000 --full-scan
cargo run -- check_sort --bam example.bam
cargo run -- check_sort --bam example.bam --sample-records 50000 --strict
cargo run -- check_index --bam example.bam
cargo run -- check_index --bam example.bam --require
cargo run -- index --bam example.bam
cargo run -- index --bam example.bam --format csi --out example.bam.csi
cargo run -- summary --bam example.bam
cargo run -- summary --bam example.bam --sample-records 250000 --include-mapq-hist --include-flags
cargo run -- summary --bam example.bam --full-scan --prefer-index
cargo run -- check_tag --tag NM --bam example.bam
cargo run -- check_tag --tag RG --require-type Z --bam example.bam --count-hits
cargo run -- check_tag --tag SA --bam example.bam --full-scan
cargo run -- validate --bam example.bam
cargo run -- validate --bam example.bam --header-only
cargo run -- validate --bam example.bam --records 100000 --include-warnings
cargo run -- checksum --bam example.bam --mode raw-record-order
cargo run -- checksum --bam example.bam --mode canonical-record-order --only-primary --mapped-only
cargo run -- checksum --bam example.bam --mode payload --include-header --exclude-tags NM,MD,AS
cargo run -- checksum --bam example.bam --mode all
cargo run -- sort --bam example.bam --out sorted.bam
cargo run -- sort --bam example.bam --out qname.bam --order queryname --queryname-suborder lexicographical
cargo run -- sort --bam example.bam --out sorted.bam --verify-checksum --create-index
cargo run -- merge --bam shard1.bam shard2.bam --out merged.bam
cargo run -- merge --bam a.bam b.bam --out merged.sorted.bam --sort --verify-checksum
cargo run -- merge --bam lane1.bam lane2.bam --out merged.qname.bam --order queryname --queryname-suborder lexicographical
```

`header` uses the binary BAM reference section as authoritative for reference
names and lengths, and joins optional fields from textual `@SQ` records into the
structured JSON view when present.

`check_map` prefers index-derived mapping summaries when a usable BAI is present.
Without a usable index it falls back to scan-based evidence. Bounded scan mode is
a fast assessment, not an exhaustive proof unless full-scan mode is used.

`check_sort` preserves declared sort metadata from the BAM header and compares it
with observed ordering in a bounded record scan. Coordinate and queryname sorts
are the primary observed modes in this slice; specialized modes such as
template-coordinate or minimiser-related sub-sorts are preserved from the header
with limited observed confirmation.

`check_index` looks for adjacent companion indices using the repository's current
priority order and reports whether a selected index is BAI, CSI, or unknown.
Stale-index detection is heuristic and based on file modification times rather
than proof that every indexed offset still matches the BAM.

`index` currently establishes the index lifecycle command path by validating the
BAM, selecting a default output path (`<bam>.bai` or `<bam>.csi`), and enforcing
overwrite rules. Actual BAI/CSI writing is still deferred, and the JSON error
response makes that limitation explicit instead of pretending an index was built.

`summary` combines BAM header metadata with a bounded scan by default and
switches to full-file totals only when EOF is actually reached or `--full-scan`
is used. When a usable BAI is available and `--prefer-index` is enabled,
index-derived mapped/unmapped totals are reported separately from scan-derived
record-category counts so the evidence source stays explicit.

`check_tag` traverses BAM auxiliary fields just deeply enough to establish tag
presence, optional type-constrained presence, or full-scan absence. In bounded
mode, a missing tag only means it was not found in the examined records. In
full-scan mode, absence is reported only when the scan reaches EOF cleanly.

`validate` is the first deeper integrity pass in the repository. It checks BAM
file/header structure, streams through records, validates record layout and aux
traversal, and reports findings as `error`, `warning`, or `info`. Header-only
and bounded-record modes are supported, and finding-bearing invalid BAMs still
return structured validation payloads instead of collapsing to an opaque error.

`checksum` exposes a small number of explicit checksum domains instead of one
overloaded digest. `raw-record-order` hashes the deterministic per-record
serialization in encounter order, so it is suitable for order-sensitive stream
preservation checks. `canonical-record-order` hashes per-record canonical
serializations, sorts the per-record digests, and hashes the sorted digest list,
so it is intended for comparing BAM content across reordering operations such as
sorting. `header` hashes raw header text plus the binary reference dictionary in
order. `payload` hashes the deterministic record payload stream and can prefix
the header serialization when `--include-header` is requested.

Filters and exclusions are part of the checksum definition. `--only-primary`,
`--mapped-only`, `--include-header`, and `--exclude-tags` must match when
comparing digests. Auxiliary-tag exclusions apply to the deterministic
record-content serialization used by the record-based checksum modes in this
slice. The current order-insensitive canonical mode collects per-record digests
in memory before sorting them, which is correct and explicit but may need a
chunked or external-sort strategy for very large BAMs later.

`sort` is the first transformational command in the repository. The current
implementation reads records into memory, derives deterministic coordinate or
queryname lexicographical sort keys, rewrites the `@HD` sort metadata, and
writes a new BGZF/BAM output. Coordinate output is intended to be suitable for
standard BAM indexing. Queryname output is not suitable for standard coordinate
BAI indexing. Optional `--verify-checksum` support compares canonical
order-insensitive checksums of the input and output so content preservation can
be confirmed explicitly rather than implied.

`merge` builds on the same writer and comparator family. By default it preserves
input-file concatenation order. With `--sort` or `--order coordinate`, it reads
all records, applies the same coordinate comparator family used by `sort`, and
writes coordinate-ordered output. Queryname merge uses the same lexicographical
queryname ordering family. The current implementation is in-memory and requires
identical binary reference dictionaries across all inputs. Queryname and
input-order merge outputs are not suitable for standard coordinate BAI
indexing. Optional checksum verification compares the canonical
order-insensitive multiset checksum of the combined inputs against the merged
output.

## Development

```bash
cargo fmt
cargo clippy --all-targets -- -D warnings
cargo test
```

## Current Status

The repository now contains a production-minded shallow BAM slice with shared
JSON contracts, structured error handling, fast file probing, and real BGZF EOF
inspection. Full BAM validation and broader BAM operations will be implemented
incrementally under the project charter in
[`docs/project-charter.md`](docs/project-charter.md).

## Specification Layer

The repository now carries a dedicated `spec/` tree for governed external
contracts. It exists so the CLI surface, JSON outputs, examples, and
interoperability expectations can be reviewed and versioned as public
interfaces.

Key paths:

* `spec/jsonschema/` contains machine-readable schemas for command outputs
* `spec/examples/` contains canonical success and failure JSON examples
* `spec/cli/` contains command, option, and exit-code contracts
* `spec/contracts/` documents versioning, compatibility, and naming rules
* `tests/contract/` contains contract-test scaffolding

When a pull request changes Bamana’s external contract, it should update:

* the relevant schema file
* the canonical examples
* the CLI/docs contract pages
* the contract tests or fixtures when applicable

Run the contract scaffolding with:

```bash
cargo test --test contract
```

Golden/example updates should be intentional and reviewable. Treat schema field
renames, enum-literal changes, nullability changes, and meaning changes as
breaking until explicitly reviewed under the contract versioning rules in
`spec/contracts/versioning.md`.

## Fixture Suite

The repository also carries a planned tiny synthetic fixture suite under
`tests/fixtures/`. This is the path from schema-only contract checks to
executable interop tests against real BAM and BAI inputs.

Key fixture assets:

* `tests/fixtures/manifest.json` defines the planned fixture inventory
* `tests/fixtures/plans/` documents taxonomy, coverage, and regeneration
* `tests/fixtures/expected/` is reserved for fixture-backed golden JSON outputs
* `docs/fixtures.md` explains how fixture changes should be reviewed

The first fixture suite is intentionally small and deterministic. It favors a
few purpose-built BAMs over large downloaded datasets or opaque binary blobs.
