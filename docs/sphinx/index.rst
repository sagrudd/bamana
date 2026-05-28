Bamana Benchmark Profiles
=========================

This Sphinx site documents the owned benchmark profiles exposed through
``bamana benchmark``. These profiles build the local Bamana binaries, build the
benchmark container, run the benchmark inside that container, and render a PDF
report from R Markdown.

Build the site locally with:

.. code-block:: bash

   python -m pip install -r docs/sphinx/requirements.txt
   sphinx-build -b html docs/sphinx docs/sphinx/_build/html

.. toctree::
   :maxdepth: 2
   :caption: Benchmark Profiles

   fastq_ingress
   fastq_gz_enumerate
   bgzf_microbenchmarks
   header_microbenchmarks
   scanner_microbenchmarks
   fastq_microbenchmarks

.. toctree::
   :maxdepth: 2
   :caption: Public CLI Commands

   public_commands

.. toctree::
   :maxdepth: 2
   :caption: Technical Notes

   native_bam_header_codec
   native_bam_record_scanner
   native_fastq_core
   native_command_migration
   native_inspection_validation
   native_mutation_forensics
   native_transform_ingest
   native_bam_index_random_access
   native_indexed_region_workflows
   native_indexed_region_selection
   native_extended_index_compatibility
