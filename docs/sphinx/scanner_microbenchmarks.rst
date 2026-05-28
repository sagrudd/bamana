Native BAM Scanner Microbenchmarks
==================================

Milestone 3 includes lightweight native BAM scanner microbenchmarks for local
regression checks. They do not require private input data: the benchmark binary
generates deterministic BAM fixtures with synthetic references, alignment
records, sequence and quality payloads, and selected auxiliary tags, then emits
machine-readable JSON.

Build the benchmark binary:

.. code-block:: bash

   cargo build --release --bin bamana --bin scanner_microbench

Run a small profile with command timings:

.. code-block:: bash

   target/release/scanner_microbench \
     --profile small \
     --iterations 10 \
     --bamana-bin target/release/bamana \
     --out scanner-small.json

Profiles are:

* ``small``: 1,024 records across 2 references, useful for CI and quick local
  checks
* ``medium``: 50,000 records across 8 references, useful for normal development
  comparisons
* ``large``: 250,000 records across 24 references, useful for stressing scanner
  throughput on larger synthetic BAM bodies

The JSON result contains:

* ``record_scan_throughput``: records-per-second and bytes-per-second for
  native BGZF/header opening plus complete alignment-record iteration through
  ``BamScanner``
* ``selective_field_extraction_throughput``: the same traversal plus selective
  access to core fields, read name, sequence length, and ``NM`` aux-tag
  presence
* optional ``summary``, ``check_sort``, ``check_map``, ``validate``,
  ``check_tag``, ``subsample_bam``, ``sort``, ``merge``, ``checksum``,
  ``explode``, ``consume``, ``index_bam``, ``check_index``,
  ``check_map_indexed``, ``summary_indexed``, ``select_region_scan_fallback``,
  ``select_region_indexed_output``, ``check_index_csi_detect_only``,
  ``check_map_region_csi_fallback``, ``summary_region_csi_fallback``,
  ``select_region_csi_fallback``, ``inspect_duplication``, ``deduplicate``,
  and ``forensic_inspect`` command timings when
  ``--bamana-bin`` is supplied

The scanner timings and command timings answer different questions. Scanner
timings measure the in-process substrate and selected field access. Command
timings include process startup, CLI parsing, JSON envelope emission, file
probing, and command-specific payload construction; use them for before/after
command migration checks, not as pure scanner measurements.

Milestone 6 command timings are smoke timings over deterministic synthetic BAM
input. ``check_sort`` is run in strict mode. ``check_map`` and ``summary`` run
without adjacent index sidecars, so they exercise scanner-derived evidence
rather than index-derived evidence. ``check_tag`` performs a full scan for the
synthetic ``NM`` auxiliary tag, ``validate`` runs the default full structural
pass, ``sort`` runs a coordinate rewrite with canonical checksum verification,
``merge`` runs a coordinate merge with canonical checksum verification over two
copies of the deterministic fixture, ``checksum`` runs all checksum domains
with header inclusion, ``NM`` tag exclusion, and mapped-only filtering,
``explode`` writes scanner-backed BAM contiguous shards, ``consume`` runs
scanner-backed BAM alignment ingest with explicit policy reporting,
``inspect_duplication`` runs a full ``qname-seq-qual-rg`` CLI scan over the
deterministic BAM fixture, and ``deduplicate`` runs a full dry-run
``qname-seq-qual-rg`` CLI plan.
``forensic_inspect`` runs explicit full-scan header, read-group, program-chain,
read-name, tag, and duplication-hallmark scopes. These timings do not exercise
malformed-input paths and do not imply comparator parity with external tools.
For the Milestone 8 command set, these notes intentionally distinguish process
startup and JSON emission from full-record materialization, in-memory sorting
cost, merge compatibility and merge-ordering cost, native BGZF compression
cost, checksum-domain traversal, shard planning, ingest normalization, and CRAM
compatibility behavior.

``subsample_bam`` is a command-level dry-run timing over the generated BAM
fixture. It proves the BAM ``subsample`` CLI path is runnable through the
benchmark hook, but it should be interpreted as command smoke timing rather
than output-write throughput.

``sort`` is a command-level rewrite timing over the generated BAM fixture. It
proves the current scanner-backed load, in-memory coordinate ordering, native
BGZF write, and canonical checksum verification path is runnable through the
benchmark hook, but it should be interpreted as command smoke timing rather
than external-memory sort throughput or comparator parity. The timing includes
full-record materialization, in-memory sorting cost, native BGZF compression,
output finalization, process startup, and JSON emission.

``merge`` is a command-level merge timing over two copies of the generated BAM
fixture. It proves the current scanner-backed input loading, reference
dictionary compatibility check, in-memory coordinate merge, native BGZF write,
and canonical checksum verification path is runnable through the benchmark
hook, but it should be interpreted as command smoke timing rather than
external-memory merge throughput or comparator parity. The timing includes two
full-record materialization passes over the same fixture, reference dictionary
compatibility checks, in-memory merge ordering, native BGZF compression, output
finalization, process startup, and JSON emission.

``checksum`` is a command-level checksum timing over the generated BAM fixture.
It proves the current scanner-backed record traversal, header serialization
domain, raw encounter-order record domain, canonical order-insensitive record
domain, payload domain, tag exclusion reporting, and mapped-only filtering are
runnable through the benchmark hook. It should be
interpreted as command smoke timing rather than cryptographic throughput or
semantic equivalence beyond the selected checksum domains.
The timing includes checksum-domain traversal and digest construction for the
selected domains, not external-tool parity or whole-file semantic validation.

``explode`` is a command-level BAM sharding timing over the generated BAM
fixture. It proves scanner-backed BAM record traversal, original-header
preservation, native BGZF shard writing, contiguous shard ranges, and encounter
order preservation within each shard are runnable through the benchmark hook.
It should be interpreted as command smoke timing rather than FASTQ.GZI planning
throughput, reconstruction proof, or random-access parallel inflate evidence.
The BAM hook includes contiguous shard planning, original-header preservation,
native BGZF shard compression, multi-output finalization, process startup, and
JSON emission.

``consume`` is a command-level alignment ingest timing over the generated BAM
fixture. It proves scanner-backed BAM alignment loading, explicit mode and
mixed-format policy reporting, native BGZF output writing, and deferred
checksum/index reporting are runnable through the benchmark hook. It should be
interpreted as command smoke timing rather than broad mixed-format ingest
coverage, CRAM reference-policy coverage, or post-ingest checksum verification
evidence. The timing includes BAM alignment ingest normalization, native BGZF
output compression, output finalization, process startup, and JSON emission.
CRAM compatibility behavior is deliberately not exercised by this synthetic BAM
smoke hook and remains covered by explicit reference-policy tests and
documentation.

``index_bam`` is a command-level BAM index construction timing over the
generated coordinate-sorted BAM fixture. It proves native BAI bin, chunk,
linear-index, metadata-count, sidecar-write, and output-finalization paths are
runnable through the benchmark hook. It should be interpreted as command smoke
timing rather than comparator parity, CSI writing evidence, or indexed lookup
throughput.

``check_index`` is a command-level BAI inspection timing after ``index_bam`` has
created a sidecar. It includes adjacent sidecar discovery, BAI structural
validation, timestamp compatibility assessment, process startup, and JSON
emission. It does not prove that every virtual offset in the index matches the
BAM.

``check_map_indexed`` and ``summary_indexed`` are command-level indexed
consumer timings after the generated BAI sidecar exists. They prove that first
consumers can use validated BAI mapped/unmapped metadata and keep that evidence
separate from scanner evidence. They do not exercise random-access chunk
traversal or region filtering; those remain later indexed-region work.

``check_map_region_scan_fallback`` and ``summary_region_scan_fallback`` are
command-level M10 region timings before the generated BAI sidecar exists. They
prove that ``check_map --region <REGION>`` and ``summary --region <REGION>``
can parse a region, fall back to native scan evidence, filter records by the
requested interval, and emit region-scoped JSON through the benchmark hook.

``select_region_scan_fallback`` is a command-level M11 selected-output timing
before the generated BAI sidecar exists. It proves that ``select_region`` can
parse a region, fall back to native scan traversal, filter selected records,
preserve retained raw BAM record bytes, write BGZF BAM file output through the
temporary-output path, append header provenance, and emit governed JSON through
the benchmark hook.

``check_map_region_indexed`` and ``summary_region_indexed`` are command-level
M10 indexed-region timings after ``index_bam`` has created a BAI sidecar. They
exercise index lookup, BAI chunk planning, random-access traversal, region
filtering, process startup, and JSON emission for the promoted read-only
region evidence surfaces. They do not claim selected-record output, biological
interpretation, broad comparator parity, native CRAM indexed queries, or
exhaustive validation of every index offset.

``select_region_indexed_output`` is a command-level M11 selected-output timing
after ``index_bam`` has created a BAI sidecar. It exercises region parsing, BAI
chunk planning, random-access traversal, selected-record filtering, duplicate
suppression, raw-record preservation, BGZF BAM output writing, temporary-output
finalization, header provenance, index-invalidation reporting, process startup,
and JSON emission. It does not claim binary stdout output, public
``--region-file`` behavior, replacement output-index creation, biological
interpretation, broad comparator parity, native CRAM indexed queries, or
exhaustive validation of every index offset.

``check_index_csi_detect_only``, ``check_map_region_csi_fallback``,
``summary_region_csi_fallback``, and ``select_region_csi_fallback`` are
command-level M12 index compatibility smoke timings. The benchmark removes the
generated BAI sidecar, writes a minimal CSI header sidecar, and exercises
detect-only support-level reporting plus native scan fallback with CSI context
preserved in JSON. ``select_region_csi_fallback`` additionally verifies
selected-output fallback with ``input_index`` compatibility reporting. These
rows are guardrails for detect-only support-level reporting and native scan
fallback; they do not claim CSI bin parsing, CSI chunk planning, CSI
random-access traversal, CSI writing, or large-reference CSI support.

``inspect_duplication`` is a command-level inspection timing over the generated
BAM fixture. It proves the native scanner-backed duplication inspection CLI path
is runnable through the benchmark hook, but it should be interpreted as command
smoke timing rather than duplicate-detection sensitivity.

``deduplicate`` is a command-level dry-run timing over the generated BAM
fixture. It proves the conservative remediation CLI can build a native
scanner-backed plan through the benchmark hook, but it should be interpreted as
command smoke timing rather than applied output-write throughput.

``forensic_inspect`` is a command-level inspection timing over the generated
BAM fixture. It proves the native header and scanner-backed provenance
inspection CLI path is runnable through the benchmark hook, but it should be
interpreted as command smoke timing rather than forensic sensitivity.

Milestone 7 scanner-backed command timings distinguish scan cost from rewrite
cost. ``inspect_duplication`` and ``forensic_inspect`` perform inspection-only
scan paths and never write output artifacts. ``deduplicate`` is run in
dry-run mode, so it includes process startup, file probing, scanner traversal,
planning, and JSON emission, but it does not include applied rewrite cost,
output BGZF compression cost, removed-report writing, or checksum verification
cost. These command timings are for regression smoke evidence and do not imply
comparator parity with external duplicate-marking, remediation, or provenance
tools.

Milestone 10 indexed-region timings distinguish scan fallback, index lookup,
BAI chunk planning, random-access traversal, region filtering, command startup,
and JSON emission. They are smoke timings over deterministic synthetic BAM
fixtures, not comparator-parity claims, native CRAM indexed-query support, or
biological interpretation.

Milestone 11 selected-region output timings distinguish scan fallback, BAI
chunk planning, random-access traversal, selected-record filtering, duplicate
suppression, raw-record preservation, BGZF BAM file writing, temporary-output
finalization, header provenance, index-invalidation reporting, command startup,
and JSON emission. They are smoke timings over deterministic synthetic BAM
fixtures, not stdout-output evidence, public region-file evidence,
replacement output-index evidence, comparator-parity claims, native CRAM
indexed-query support, or biological interpretation.

Results conform to
``benchmarks/results/scanner_microbench.schema.json`` and can be archived
beside other benchmark result artifacts.

Milestone 3 Closeout
--------------------

The Milestone 3 closeout ran the ``small`` profile with one iteration and a
JSON smoke check. The smoke check verified the selected profile, iteration
count, 1,024 generated records, and the expected result schema. Longer local
runs should increase ``--iterations`` and use ``medium`` or ``large`` when
comparing scanner throughput across changes.
