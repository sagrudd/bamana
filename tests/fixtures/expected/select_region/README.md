# select_region Expected Outputs

This directory is reserved for governed `select_region` expected-output
fixtures once the tiny BAM/BAI files are materialized.

Planned scenarios:

* indexed selected-output success from `tiny.valid.coordinate` plus
  `tiny.valid.coordinate.bai`;
* scan fallback when no usable adjacent BAI is available;
* duplicate and overlapping CLI-region suppression;
* output-index sidecar collision without `--force`;
* forced stale-sidecar removal with `output.index_invalidation` reporting;
* same-path input/output rejection.

The current fixture plan covers BGZF BAM file output only. Binary stdout output
and public `--region-file` remain deferred.
