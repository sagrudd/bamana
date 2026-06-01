Release Hardening And Public Contract Freeze
============================================

Milestone 15 is active as of 2026-06-01. M15.1 activates release hardening
after Milestone 14 closeout. This activation is a scope-freeze step: it does
not add commands, change command behavior, change JSON schemas, or promote
benchmark/comparator claims.

Release Boundary Inventory
--------------------------

The starting release boundary accepts the currently implemented and documented
CLI commands:

* ``benchmark``
* ``identify``
* ``enumerate``
* ``subsample``
* ``inspect_duplication``
* ``deduplicate``
* ``forensic_inspect``
* ``annotate_rg``
* ``consume``
* ``explode``
* ``fastq``
* ``checksum``
* ``merge``
* ``reheader``
* ``sort``
* ``select_region``
* ``unmap``
* ``verify``
* ``check_eof``
* ``header``
* ``check_map``
* ``check_index``
* ``index``
* ``summary``
* ``validate``
* ``check_tag``
* ``check_sort``

Public Contract Commands
------------------------

``benchmark``, ``fastq``, and ``unmap`` remain named public contract commands.
They must retain governed schemas, canonical examples, CLI documentation,
Sphinx documentation, contract tests, and benchmark/no-claim notes where
applicable.

Release Boundary Limits
-----------------------

Acceptance into the M15 release boundary means each command is eligible for
release hardening only as currently specified. It does not imply broad
external-tool comparator parity, release performance promises, biological
equivalence, CRAM indexed-query behavior, CSI writing, native CRAM parsing or
writing, public ``select_region --region-file``, binary stdout output for
selected records, replacement output-index creation, or unmeasured benchmark
claims for ``fastq``, ``unmap``, or ``identify``.

Later M15 tasks must freeze, document, or explicitly exclude each
release-facing surface before the release hardening milestone can close.
