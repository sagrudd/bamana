# Benchmark Tools

This directory contains the governance layer for benchmark comparator
representation.

It separates three concepts that should not be blurred:

* tool identity: which tool is being benchmarked
* workflow variant: what exact operation chain that tool runs in a scenario
* wrapper implementation: how the benchmark framework invokes it

Files in this directory:

* [tool_wrapper_contract.md](/Users/stephen/Projects/bamana/benchmarks/tools/tool_wrapper_contract.md): wrapper responsibilities and status semantics
* [workflow_variant_matrix.md](/Users/stephen/Projects/bamana/benchmarks/tools/workflow_variant_matrix.md): supported and unsupported tool/scenario combinations
* [tool_registry.schema.json](/Users/stephen/Projects/bamana/benchmarks/tools/tool_registry.schema.json): machine-readable tool registry schema
* [tool_registry.example.json](/Users/stephen/Projects/bamana/benchmarks/tools/tool_registry.example.json): initial registry content for Bamana and comparators
* [wrappers/README.md](/Users/stephen/Projects/bamana/benchmarks/tools/wrappers/README.md): documented wrapper examples and conventions

The current Nextflow pipeline uses per-tool module processes plus the common
timing wrapper [../bin/run_benchmark.sh](/Users/stephen/Projects/bamana/benchmarks/bin/run_benchmark.sh).
This directory documents that contract explicitly so the benchmark framework
remains auditable.
