# Expected Outputs For `reheader`

This directory reserves governed expected-output fixtures for M7 `reheader`
coverage. `reheader` fixtures must preserve the command's header-only contract:
they may change BAM header metadata, but they must not claim per-record `RG:Z`
mutation.

Reserved output names:

* `reheader.<fixture-id>.dry_run.success.json`
* `reheader.<fixture-id>.rewrite.success.json`
* `reheader.<fixture-id>.failure.json`

Expected-output assertions should cover mutation operations, planning fields,
execution mode, output/index reporting, checksum reporting, and notes that
distinguish `reheader` from `annotate_rg`.
