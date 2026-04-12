#!/usr/bin/env bash
set -euo pipefail

# Maintainer-oriented generation recipe for the first tiny CRAM provenance
# package. This script documents the intended derivation path; it is not used by
# CI automatically.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_DIR="$ROOT_DIR/source"
BAM_DIR="$ROOT_DIR/bam/valid"
CRAM_DIR="$ROOT_DIR/cram/valid"

SOURCE_SAM="$SOURCE_DIR/tiny.valid.cram.explicit_ref.source.sam"
REFERENCE_FASTA="$SOURCE_DIR/tiny.ref.primary.fasta"
DERIVED_BAM="$BAM_DIR/tiny.valid.cram.explicit_ref.source.bam"
DERIVED_CRAM="$CRAM_DIR/tiny.valid.cram.explicit_ref.cram"

mkdir -p "$BAM_DIR" "$CRAM_DIR"

cat <<'EOF'
This recipe assumes a reviewed external CRAM-capable toolchain such as samtools.
The provenance root is the committed SAM and FASTA, not the derived BAM/CRAM.

Recommended conceptual pipeline:
  1. Verify that the SAM @SQ dictionary matches the FASTA exactly.
  2. Derive BAM from SAM.
  3. Derive CRAM from SAM or BAM using the explicit FASTA.
  4. Verify that the CRAM decodes successfully with the FASTA.
  5. Verify that strict-policy consume should fail when the FASTA is withheld.
EOF

if ! command -v samtools >/dev/null 2>&1; then
  echo "samtools was not found on PATH; this script documents the recipe but cannot run it." >&2
  exit 1
fi

samtools view -b -o "$DERIVED_BAM" "$SOURCE_SAM"
samtools view -C -T "$REFERENCE_FASTA" -o "$DERIVED_CRAM" "$SOURCE_SAM"
samtools quickcheck "$DERIVED_BAM" "$DERIVED_CRAM"

cat <<EOF
Derived BAM:  $DERIVED_BAM
Derived CRAM: $DERIVED_CRAM

Manual verification expectations:
  * decode the CRAM with the explicit FASTA and confirm success
  * run the strict-policy missing-reference consume scenario and confirm failure
  * review binary diffs carefully if the external toolchain version changed
EOF
