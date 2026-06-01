# Command Evidence Matrix

M14.2 defines the current benchmark and comparator evidence level for each
public Bamana command. This matrix is a scope control document: it records
where evidence exists and where Bamana makes no external comparator claim.
M14.5 adds `benchmarks/comparator_profiles.json` as the aligned measured
profile catalog for public-profile comparator rows; it does not change any
no-claim command rows.

Evidence levels:

* `public_profile_comparator`: measured only through a public
  `bamana benchmark --profile` profile and only for the named scenario.
* `scenario_matrix_comparator_scaffold`: represented in the workflow variant
  matrix, but not promoted as release-facing comparator evidence by M14.2.
* `local_smoke`: measured only through repository-local microbenchmark command
  timings against deterministic fixtures.
* `no_external_comparator_claim`: no current benchmark, smoke hook, or
  comparator claim for this command or command mode.

| Command | Current Evidence Level | Evidence Source | Comparator Scope | M14.2 Boundary |
| --- | --- | --- | --- | --- |
| `benchmark` | `local_smoke` | contract tests plus public profile routing for `fastq_ingress` and `fastq_gz_enumerate` | Profile runner only; not itself a biological or tool-output comparator | Public contract command; profile output scope is governed per profile |
| `identify` | `no_external_comparator_claim` | CLI/schema/docs only | none | Format sniffing has no external comparator claim |
| `enumerate` | `public_profile_comparator` for FASTQ.GZ, `local_smoke` for FASTQ/FASTQ.GZ, otherwise `no_external_comparator_claim` | `fastq_gz_enumerate`; `fastq_microbench` rows `enumerate_fastq` and `enumerate_fastq_gz` | FASTQ.GZ record counting against gzip decompression plus line counting | BAM, SAM, and FASTA enumeration have no comparator claim |
| `subsample` | `local_smoke`; `scenario_matrix_comparator_scaffold` for BAM and FASTQ.GZ subsampling | `scanner_microbench` row `subsample_bam`; `fastq_microbench` rows `subsample_fastq` and `subsample_fastq_gz`; workflow variant matrix rows for samtools, sambamba, and seqtk | Scaffolded tool comparisons only; not release-facing comparator parity | Random policy, deterministic identity, and output equivalence remain command-specific |
| `inspect_duplication` | `local_smoke` | `scanner_microbench` row `inspect_duplication` | none | Evidence is provenance/collection smoke timing, not duplicate-marking parity |
| `deduplicate` | `local_smoke` | `scanner_microbench` row `deduplicate` | none | No Picard/GATK duplicate-marking comparator claim |
| `forensic_inspect` | `local_smoke` | `scanner_microbench` row `forensic_inspect` | none | No fraud-detection, validation, or external provenance-tool claim |
| `annotate_rg` | `local_smoke` | `header_microbench` row `annotate_rg` | none | Record-level read-group mutation has no external comparator claim |
| `consume` | `public_profile_comparator` for FASTQ.GZ unmapped ingest; `local_smoke` for synthetic BAM alignment ingest; otherwise `no_external_comparator_claim` | `fastq_ingress`; `scanner_microbench` row `consume`; workflow matrix `fastq_consume_pipeline` | FASTQ.GZ-to-unmapped-BAM against fastcat-plus-samtools only | CRAM, SAM, mixed-directory, cache-backed, and biological equivalence claims remain out of scope |
| `explode` | `local_smoke` | `scanner_microbench` row `explode` | none | No shard equivalence, reconstruction-throughput, or parallel-inflate comparator claim |
| `fastq` | `no_external_comparator_claim` | CLI/schema/docs only | none | Public contract command; no benchmark hook or external comparator claim yet |
| `checksum` | `local_smoke` | `scanner_microbench` row `checksum` | none | Checksum domains are Bamana-owned, not external tool parity |
| `merge` | `local_smoke` | `scanner_microbench` row `merge` | none | No external-memory merge throughput or comparator claim |
| `reheader` | `local_smoke` | `header_microbench` row `reheader` | none | Header-only mutation semantics are Bamana-owned |
| `sort` | `local_smoke`; `scenario_matrix_comparator_scaffold` for mapped BAM pipelines | `scanner_microbench` row `sort`; workflow variant matrix `mapped_bam_pipeline` | Scaffolded against samtools and sambamba paths, not release-facing comparator parity | Full external-memory sort performance and comparator parity remain unclaimed |
| `select_region` | `local_smoke` | `scanner_microbench` rows `select_region_scan_fallback`, `select_region_indexed_output`, and `select_region_csi_fallback` | none | No public region-file, stdout-output, replacement-index, biological, CRAM, or broad comparator claim |
| `unmap` | `no_external_comparator_claim` | CLI/schema/docs only | none | Public contract command; no benchmark hook or external comparator claim yet |
| `verify` | `local_smoke` | `bgzf_microbench` and `header_microbench` row `verify` | none | Shallow Bamana-native validation, not external validator parity |
| `check_eof` | `local_smoke` | `bgzf_microbench` row `check_eof` | none | EOF-marker evidence only |
| `header` | `local_smoke` | `header_microbench` row `header` | none | Native header parse/serialization smoke, not external parser parity |
| `check_map` | `local_smoke` | `scanner_microbench` rows `check_map`, `check_map_indexed`, `check_map_region_scan_fallback`, `check_map_region_indexed`, and `check_map_region_csi_fallback` | none | Mapping evidence stays index/scan-source-specific |
| `check_index` | `local_smoke` | `scanner_microbench` rows `check_index` and `check_index_csi_detect_only` | none | BAI/CSI support-level evidence only; no CSI traversal/writing claim |
| `index` | `local_smoke` | `scanner_microbench` row `index_bam` | none | BAI creation smoke only; no CSI writing or external indexer parity |
| `summary` | `local_smoke` | `scanner_microbench` rows `summary`, `summary_indexed`, `summary_region_scan_fallback`, `summary_region_indexed`, and `summary_region_csi_fallback` | none | Operational summary evidence stays index/scan-source-specific |
| `validate` | `local_smoke` | `scanner_microbench` row `validate` | none | Structural validation smoke only, not external validator parity |
| `check_tag` | `local_smoke` | `scanner_microbench` row `check_tag` | none | Aux-tag presence evidence only |
| `check_sort` | `local_smoke` | `scanner_microbench` row `check_sort` | none | Declared/observed sort evidence only |

M14.2 does not add benchmark profiles, result schemas, fixture generation, or
runtime command behavior. It does not promote workflow-matrix scaffold rows to
release-facing comparator evidence. Later M14 tasks must attach
command-specific fixtures, semantic assumptions, and result-schema coverage
before any additional comparator claim is made.

CRAM consume behavior, unmeasured command modes, and scaffold-only workflow rows
remain no external comparator claim surfaces.
