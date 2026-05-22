# Expected Outputs For `annotate_rg`

This directory reserves governed expected-output fixtures for M7 `annotate_rg`
coverage. `annotate_rg` fixtures must preserve the command's record-level
contract: it scans and rewrites alignment records to apply explicit `RG:Z`
annotation policy, rather than acting as a header-only mutation command.

Reserved output names:

* `annotate_rg.<fixture-id>.only_missing.success.json`
* `annotate_rg.<fixture-id>.replace_existing.success.json`
* `annotate_rg.<fixture-id>.fail_on_conflict.failure.json`

Expected-output assertions should cover request mode, header policy, record
summary counts, output/index reporting, RG-excluded checksum reporting, and
notes that distinguish `annotate_rg` from `reheader`.
