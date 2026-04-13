# Support Matrix Contract

## Purpose

The support matrix is the benchmark reporting layer that explains whether a
tool and scenario combination:

* is intended to be supported
* was attempted in the current benchmark run set
* succeeded
* failed
* is explicitly unsupported

Timing plots alone are not sufficient. Missing bars can mean:

* unsupported by design
* supported but not attempted
* supported but failed

The support matrix makes those cases explicit.

## Intended Support Versus Observed Outcome

The support matrix joins two sources of truth:

1. capability metadata from the benchmark tool registry and workflow-variant
   matrix
2. observed run outcomes from the tidy benchmark result rows

These are related but not identical.

Examples:

* a combination can be intended to be supported but fail in practice
* a combination can be intentionally unsupported
* a supported combination can be omitted from one specific run set and should
  therefore appear as `not_attempted`

## Support Status Taxonomy

The first support-matrix slice uses this taxonomy:

* `supported_success`
  means the combination is intended to be supported and at least one measured
  run succeeded
* `supported_failed`
  means the combination is intended to be supported, was attempted, and no
  successful run was observed
* `unsupported`
  means the combination is explicitly unsupported or not applicable according
  to the capability layer
* `not_attempted`
  means the combination may be supported in principle but was not included in
  the current benchmark run set
* `mixed_results`
  means the combination had both successful and failed attempted runs

Hard rules:

* unsupported is not failure
* not_attempted is not unsupported
* supported_failed represents a real execution problem worth investigation

## Machine-Readable Support Matrix

`support_matrix.csv` contains one row per:

* tool
* scenario
* workflow variant

Required columns:

* `tool`
* `tool_version`
* `scenario`
* `workflow_variant`
* `input_type`
* `intended_support`
* `attempted`
* `n_runs`
* `n_success`
* `n_failed`
* `n_unsupported`
* `support_status`
* `notes`

### Column Semantics

* `intended_support`
  comes from the capability layer and is typically `supported`, `unsupported`,
  or `planned`
* `attempted`
  indicates whether the current benchmark run set contained any measured row
  for that combination
* `n_runs`, `n_success`, `n_failed`, `n_unsupported`
  describe observed outcomes in the current run set
* `support_status`
  is the derived reporting label used in tables and plots

## Human-Friendly Support Summary

`support_summary.csv` contains one row per:

* tool
* scenario

It provides a compact reporting view for publication or presentation.

Recommended use:

* place the support summary beside timing or throughput figures
* use it to explain missing bars or intentionally absent comparisons

## Interpretation Rules

When reading benchmark figures:

* consult the support matrix alongside timing plots
* do not treat unsupported combinations as slow results
* do not treat not-attempted combinations as unsupported
* investigate `supported_failed` combinations before drawing performance
  conclusions

## Capability Source

The capability source must remain auditable.

The current support matrix derives intended support from:

* [../tools/tool_registry.example.json](/Users/stephen/Projects/bamana/benchmarks/tools/tool_registry.example.json)
* [../tools/workflow_variant_matrix.md](/Users/stephen/Projects/bamana/benchmarks/tools/workflow_variant_matrix.md)

Observed status derives from:

* [result.schema.json](/Users/stephen/Projects/bamana/benchmarks/results/result.schema.json)
* [tidy_result_contract.md](/Users/stephen/Projects/bamana/benchmarks/results/tidy_result_contract.md)

These terms should remain harmonized. The support matrix must not invent a
status vocabulary that drifts away from wrapper capability or tidy result
semantics.
