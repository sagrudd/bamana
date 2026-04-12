# Bamana Project Charter

## 1. Project Title

**Bamana**  
A high-performance Rust toolkit for verification, quality control, inspection, and transformation of BAM files and related bioinformatics formats.

## 2. Project Sponsor

**Mnemosyne Biosciences Ltd**

Mnemosyne Biosciences Ltd is the formal sponsor of the Bamana project and is responsible for its strategic direction, stewardship, and release governance.

## 3. Mission

The mission of **Bamana** is to provide a **focused, high-performance, automation-friendly bioinformatics toolkit** for use in **regulated, controlled, and operationally demanding environments**.

Bamana is being developed to deliver a set of carefully scoped capabilities for BAM and related file formats with an emphasis on:

* **performance**
* **correctness**
* **determinism**
* **auditability**
* **clear machine-readable outputs**
* **operational suitability in compliance-sensitive settings**

The project is intentionally designed to be a **more performant toolkit than htslib / hts_lib-based alternatives for a defined set of common and high-value BAM operations**. This objective is central to the project and is not a secondary optimization goal.

## 4. Background and Naming

The name **Bamana** was selected as a deliberate and memorable play on **BAM**, the Binary Alignment/Map format, combined with the cadence of the word **banana** to create a distinctive project identity.

The name is intended solely as a technical and linguistic reference to the BAM file format. It is **not** intended as a reference to, appropriation of, or commentary on the Bamana people, language, or culture. No disrespect is intended, and project materials should state this plainly wherever the name is introduced.

## 5. Problem Statement

Existing bioinformatics tooling for BAM processing is mature and widely used, but many common implementations prioritize broad feature coverage, historical compatibility, or general-purpose utility over minimal latency and tightly scoped operational speed.

In regulated and controlled environments, there is a recurring need for tooling that can:

* determine file identity quickly and reliably
* distinguish shallow verification from deep validation
* inspect mapping and sorting state rapidly
* detect truncation and EOF completeness
* summarise alignment characteristics in structured form
* transform, partition, merge, and sort BAM files efficiently
* ingest BAM-, SAM-, and FASTQ-like upstream inputs into disciplined BAM normalization workflows
* integrate cleanly with workflow engines, audit pipelines, and machine-based validation gates

Bamana exists to address this need with a deliberately narrow and disciplined design under the sponsorship of **Mnemosyne Biosciences Ltd**.

## 6. Vision

Bamana will become a trusted toolkit for **fast-path BAM interrogation and transformation** in environments where software behavior must be predictable, measurable, and suitable for operational governance.

The project vision is to establish Bamana as the preferred tool when users need:

* **clear semantics**
* **stable command behavior**
* **JSON-native outputs**
* **reproducible performance**
* **high throughput on routine BAM operations**

## 7. Scope

### 7.1 In Scope

The initial scope of Bamana includes development of a Rust-based command-line toolkit centered on BAM files, with subcommands in the following categories:

* file type identification
* shallow BAM verification
* EOF completeness checking
* mapping-state inspection
* sort-state inspection
* BAM tag inspection
* BAM summary metrics
* BAM splitting / explosion for distributed workflows
* BAM merge
* BAM sort
* BAM header-only metadata mutation
* BAM record-level read-group annotation
* BAM, FASTQ, and FASTQ.GZ subsampling under explicit deterministic or random policy
* upstream file and directory ingestion into normalized BAM
* index-aware operations
* header and reference inspection
* validation and integrity-related utilities
* JSON output contracts for all primary commands

Representative initial commands may include:

* `bamana identify`
* `bamana verify`
* `bamana consume`
* `bamana annotate_rg`
* `bamana subsample`
* `bamana reheader`
* `bamana check_eof`
* `bamana check_map`
* `bamana check_sort`
* `bamana check_tag`
* `bamana summary`
* `bamana explode`
* `bamana merge`
* `bamana sort`
* `bamana index`
* `bamana check_index`
* `bamana validate`
* `bamana checksum`
* `bamana header`

### 7.2 Out of Scope for Initial Releases

The following are explicitly out of scope for early project phases unless separately approved by **Mnemosyne Biosciences Ltd**:

* full replacement of samtools or broad htslib ecosystems
* exhaustive support for every legacy edge case before core stability is reached
* graphical interfaces
* interactive exploratory analysis features
* broad statistical reporting beyond operational QC essentials
* feature breadth that compromises performance-first design
* support for all bioinformatics formats at parity from the outset

### 7.3 Future Expansion

Future releases may extend support to adjacent formats such as:

* SAM
* CRAM beyond the current explicit consume-stage ingestion slice
* FASTQ
* FASTQ.GZ
* FASTA
* BED
* GFF

Such expansion should occur only where it does not weaken the project’s core identity: **small surface area, explicit semantics, and superior performance on critical workflows**.

Where adjacent formats are supported, the preferred operational model is
explicit normalization into BAM through governed ingest semantics rather than
an uncontrolled widening of the public data model.

## 8. Objectives

The primary objectives of the Bamana project are:

1. **Deliver a performant Rust implementation** of common BAM verification, inspection, and transformation operations.
2. **Exceed the performance of htslib / hts_lib-based toolkits** for the subset of operations Bamana explicitly targets.
3. **Provide stable JSON-based interfaces** suitable for workflow orchestration, compliance gates, and automated auditing.
4. **Separate command semantics clearly**, especially between identification, verification, completeness checks, validation, inspection, and transformation.
5. **Support deterministic and reproducible behavior** across supported environments.
6. **Enable safe adoption in regulated environments** through documentation, version discipline, benchmarking, and test coverage.

## 9. Design Principles

Bamana shall be governed by the following design principles.

### 9.1 Performance First

Each command shall pursue the fastest realistic path to a correct result. Full-file scans should not be performed where a shallower but semantically sufficient method exists.

### 9.2 Limited Surface, Strong Guarantees

Bamana shall remain intentionally narrow in feature scope, preferring a smaller number of robust capabilities over broad but weakly specified functionality.

### 9.3 Explicit Semantics

Commands shall clearly communicate what they prove, what they do not prove, and when deeper validation is required.

### 9.4 JSON-Native by Default

Primary outputs shall be structured and machine-readable. Human-readable terminal output may exist, but JSON is the core interface contract.

### 9.5 Determinism

Given identical inputs, versions, and execution conditions, Bamana should produce consistent outputs and diagnostics.

### 9.6 Compliance-Oriented Engineering

The software shall be suitable for environments requiring traceability, reproducibility, and operational controls.

## 10. Governance

### 10.1 Ownership

Bamana is owned, sponsored, and stewarded by **Mnemosyne Biosciences Ltd**.

### 10.2 Sponsorship Responsibility

As project sponsor, **Mnemosyne Biosciences Ltd** is responsible for:

* approving overall project scope
* setting strategic priorities
* defining acceptable quality and compliance expectations
* authorizing release readiness
* approving material changes to public guarantees, interfaces, and positioning

### 10.3 Maintainers

Project maintainers act under the stewardship of **Mnemosyne Biosciences Ltd** and are responsible for:

* architecture and implementation decisions
* release management
* schema and CLI stability
* benchmarking methodology
* test coverage standards
* documentation quality
* change control for externally visible behavior

### 10.4 Change Management

Any change that affects one or more of the following shall require explicit review:

* command names
* command semantics
* JSON schemas
* exit code behavior
* performance claims
* benchmark methodology
* validation guarantees
* deterministic behavior expectations

### 10.5 Versioning

The project should use clear semantic versioning or a similarly disciplined release strategy. Changes that alter output contracts, semantics, or user-visible guarantees must be documented explicitly in release notes.

### 10.6 Decision Criteria

Technical decisions should be guided in priority order by:

1. correctness
2. semantic clarity
3. performance
4. determinism
5. maintainability
6. breadth of feature coverage

## 11. Quality Standards

Bamana shall be developed to meet the following quality expectations:

* strong unit and integration test coverage
* clear error categorization
* reproducible builds where practical
* documented command semantics
* versioned JSON output expectations
* benchmark reproducibility
* robust behavior for both nominal and failure cases
* graceful handling of ambiguity and malformed input

The project should clearly distinguish:

* identification
* shallow verification
* EOF completeness checks
* deep validation
* inspection
* transformation

No command should imply a stronger guarantee than it actually provides.

## 12. Performance Position and Benchmarking

Bamana is explicitly intended to be **more performant than htslib / hts_lib-based tooling** for the subset of BAM operations it targets.

This claim shall be supported through transparent and reproducible benchmarking against representative existing tools and workflows. Performance claims must be:

* specific
* measurable
* versioned
* reproducible
* tied to defined workloads and datasets

Benchmarking should include, where relevant:

* wall-clock time
* CPU utilization
* memory usage
* I/O behavior
* replicated runs with warmup policy
* seeded random or deterministic subsampling where sampling is part of the
  workload
* indexed and unindexed cases
* small, medium, and large BAM files
* local and realistic production-style storage scenarios where possible

The repository benchmark framework is expected to remain reproducible and
containerized. `samtools` is the canonical BAM baseline. `fastcat` should be
included explicitly for ONT-style ingestion and concatenation comparisons.
Additional comparators such as `sambamba`, `seqtk`, and `rasusa` should be
used where their scope matches the benchmarked operation.

## 13. Security and Operational Considerations

Although Bamana is not primarily a security product, it shall be engineered with operational resilience in mind, including:

* safe handling of malformed files
* predictable failure modes
* no silent downgrade of guarantees
* explicit signaling of uncertainty or incomplete validation
* careful resource usage under large-file workloads

## 14. Acceptance Criteria

A release candidate for Bamana should not be considered acceptable unless all of the following are satisfied.

### 14.1 Functional Acceptance

* Core commands compile and run correctly on supported platforms.
* Each command has documented inputs, outputs, exit behavior, and semantics.
* JSON output is stable and conforms to documented schemas.
* Commands correctly distinguish between supported guarantees such as verification, EOF checking, and validation.

### 14.2 Performance Acceptance

* Benchmark results are recorded for the targeted command set.
* Performance comparisons against designated baseline tools are documented.
* For the selected operations, Bamana demonstrates meaningful performance advantage or a clearly documented rationale where parity is temporarily acceptable.

### 14.3 Reliability Acceptance

* Unit and integration tests pass in continuous integration.
* Failure cases are tested explicitly.
* Deterministic behavior is demonstrated for supported workflows.

### 14.4 Documentation Acceptance

* Repository documentation explains project purpose, scope, and non-goals.
* The naming rationale and respectful disclaimer are included.
* The role of **Mnemosyne Biosciences Ltd** as sponsor and owner is stated clearly.
* Each command is documented with examples and output expectations.
* Benchmark methodology is documented.

### 14.5 Governance Acceptance

* Versioning and release notes are in place.
* Maintainer review has occurred for public command and schema changes.
* Any known limitations are documented plainly.

## 15. Non-Goals

Bamana is not intended, at least initially, to:

* reproduce the full breadth of samtools or htslib ecosystems
* prioritize broad compatibility over clear semantics and speed
* conceal uncertainty behind permissive heuristics
* add features that materially compromise performance-first design
* optimize for every use case at the expense of regulated operational workflows

## 16. Repository Guidance

This charter should be treated as a governing project document and should live near the top level of the repository, alongside:

* `README.md`
* `LICENSE`
* `CONTRIBUTING.md`
* benchmark documentation
* command/interface specifications
* release notes

It should be reviewed whenever the project’s scope, guarantees, sponsor expectations, or performance position changes materially.

## 17. Statement of Intent

Bamana is a deliberately narrow, high-discipline bioinformatics toolkit for BAM-centric workflows.

Its purpose is to do a small number of operationally important things **quickly, correctly, and unambiguously**, with outputs that are suitable for machines, workflows, and controlled environments.

The standard for the project is not feature sprawl.  
The standard is this:

* **clear semantics**
* **reliable behavior**
* **structured outputs**
* **measurable performance**
* **and performance exceeding conventional htslib-based tooling for the operations Bamana chooses to own**

Under the sponsorship of **Mnemosyne Biosciences Ltd**, Bamana is intended to be developed and maintained as a disciplined, high-performance software project suitable for serious operational use.
