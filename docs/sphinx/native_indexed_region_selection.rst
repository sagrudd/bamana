Native Indexed Region Selection
===============================

Milestone 11 is active as of 2026-05-28. It follows the completed Milestone 10
read-only indexed-region evidence surface.

Scope
-----

M11 is the contract runway for selected-record indexed region output and
region-file input. M11.1 freezes the selection surface decision as a new
planned public command named ``select_region``. M11 must not overload
``check_map``, ``summary``, or ``subsample`` for selected-record output.

Activation Boundary
-------------------

M11.1 does not implement new CLI behavior. No public ``select_region`` synopsis
exists until later M11 tasks define:

* output semantics for stdout, files, and dry runs;
* header preservation and ``@HD`` sort-order behavior;
* record ordering;
* duplicate-region and overlapping-region behavior;
* region-file syntax and validation;
* index invalidation or regeneration notes;
* write-safety and collision handling.

Until those contracts are complete, ``check_map --region <REGION>`` and
``summary --region <REGION>`` remain the only public region-aware behavior, and
they remain read-only evidence surfaces.

Inherited Substrate
-------------------

M11 inherits:

* ``src/bam/region.rs`` for bounded region string parsing and normalization;
* ``src/bam/region_plan.rs`` for validated BAI chunk planning;
* ``src/bam/region_traversal.rs`` for random-access traversal, interval
  filtering, and duplicate virtual-offset suppression;
* ``src/commands/check_map.rs`` and ``src/commands/summary.rs`` for read-only
  region evidence payloads;
* ``src/bam/write.rs`` and ``src/output_safety.rs`` as patterns for later
  selected-record output and collision safety.

Non-Goals
---------

M11 does not imply native CRAM indexed queries, CSI large-reference support,
biological interpretation, pileup/genotyping behavior, or broad external
comparator parity.
