# Fixture Matrix

This matrix maps the intentionally small fixture suite to command coverage.
Primary-purpose coverage should be favored over adding many overlapping files.

| Fixture ID | Primary Purpose | Secondary Purpose | Supported Commands | Path Type |
| --- | --- | --- | --- | --- |
| `tiny.valid.coordinate` | Baseline valid coordinate BAM | Broad shallow/deep smoke coverage | `identify`, `verify`, `check_eof`, `header`, `check_sort`, `check_map`, `check_index`, `summary`, `validate`, `checksum` | success |
| `tiny.valid.queryname` | Queryname ordering | Re-sort and merge behavior | `check_sort`, `sort`, `merge`, `checksum` | success |
| `tiny.valid.unmapped` | Unmapped-only BAM semantics | Unmapped summary behavior | `check_map`, `summary`, `validate` | success |
| `tiny.tags.nm_rg` | Stable NM/RG tag presence | Tag exclusions in checksum | `check_tag`, `checksum`, `summary` | success |
| `tiny.tags.mixed_aux_types` | Aux traversal breadth | Validation and checksum aux handling | `check_tag`, `validate`, `checksum` | success |
| `tiny.invalid.no_eof` | Missing BGZF EOF marker | Shallow-vs-deep distinction | `check_eof`, `verify`, `validate` | failure + mixed |
| `tiny.invalid.truncated_record` | Truncated record failure | Transform negative-path coverage | `validate`, `summary`, `checksum`, `sort` | failure |
| `tiny.invalid.bad_aux` | Malformed aux traversal | Canonicalization failure | `check_tag`, `validate`, `checksum` | failure |
| `tiny.invalid.header_mismatch` | Header consistency mismatch | Header extraction warning/error policy | `header`, `validate` | failure + mixed |
| `tiny.valid.coordinate.bai` | Valid adjacent BAI | Index-backed mapping/summary | `check_index`, `check_map`, `summary` | success |
| `tiny.valid.coordinate.stale_bai` | Timestamp-based stale heuristic | Index compatibility notes | `check_index` | success + warning semantics |
| `tiny.invalid.bad_bai` | Malformed index failure | Index-backed failure fallback | `check_index`, `check_map` | failure |
| `tiny.transforms.source` | Transform source BAM | Checksum baseline | `sort`, `explode`, `checksum` | success |
| `tiny.transforms.shard1` + `tiny.transforms.shard2` | Deterministic explode outputs | Merge round-trip | `explode`, `merge`, `checksum` | success |
| `tiny.transforms.merged` | Merge result | Multiset preservation verification | `merge`, `checksum` | success |

## Duplication And Forensics Matrix

| Fixture ID | Semantic Class | Primary Purpose | Supported Commands | Expected Outcome |
| --- | --- | --- | --- | --- |
| `tiny.clean.fastq` | clean | clean FASTQ baseline | `inspect_duplication`, `deduplicate` | no duplication, no-op deduplicate |
| `tiny.clean.bam` | clean | clean BAM baseline | `inspect_duplication`, `deduplicate`, `forensic_inspect` | no suspicious duplication, no-op deduplicate, clean forensic result |
| `tiny.duplicate.fastq.whole_append` | duplicate | strongest whole-file append signature | `inspect_duplication`, `deduplicate` | strong whole-append finding, deterministic removal plan |
| `tiny.duplicate.fastq.local_block` | duplicate | local repeated contiguous block | `inspect_duplication`, `deduplicate` | local block finding, local block removal plan |
| `tiny.duplicate.bam.local_block` | duplicate | BAM local repeated block | `inspect_duplication`, `deduplicate`, `forensic_inspect` | BAM duplicate-block finding, optional suspicious hallmark overlap |
| `tiny.forensic.bam.rg_pg_inconsistent` | forensic | header provenance anomaly | `forensic_inspect` | read-group/program-chain findings without parse failure |
| `tiny.forensic.bam.readname_shift` | forensic | mixed run or concatenation hallmark | `forensic_inspect` | read-name regime-shift finding |
| `tiny.forensic.bam.concatenated_signature` | forensic | strongest suspicious concatenation case | `forensic_inspect`, `inspect_duplication` | high-confidence suspicious assessment plus duplicate hallmark overlap |
| `tiny.invalid.fastq.truncated` | invalid | FASTQ parse-failure path | `inspect_duplication`, `deduplicate` | parse uncertainty / failure |
| `tiny.invalid.bam.truncated_record.duplication` | invalid | BAM parse-failure path | `inspect_duplication`, `deduplicate`, `forensic_inspect` | parse uncertainty / failure |

## Guidance

* Use the smallest fixture that exercises the intended contract.
* Reuse baseline fixtures before adding a new one.
* If a fixture supports a command only indirectly, mark that as secondary in the
  manifest rather than inflating the matrix.
