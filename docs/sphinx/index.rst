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
