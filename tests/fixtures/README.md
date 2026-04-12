# Contract Test Fixtures

This tree holds future interoperability fixtures for Bamana contract tests.

Expected fixture categories include:

* tiny valid BAM files for shallow and deep command checks
* truncated BAMs for EOF and validation failures
* malformed-record BAMs for validation and parse-uncertainty coverage
* sorted and unsorted BAMs for `check_sort` and `sort`
* indexed and unindexed BAMs for `check_index` and `check_map`
* split/merged BAM sets for `merge`, `explode`, and checksum workflows

The current contract scaffold uses the JSON examples under `spec/examples/`
instead of runtime BAM fixtures. Real BAM fixtures can be added incrementally
without changing the test layout.
