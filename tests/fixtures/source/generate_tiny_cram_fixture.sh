#!/usr/bin/env bash
set -euo pipefail

# Maintainer-oriented generation script for the first tiny CRAM provenance
# package. This is not a CI entrypoint. It is a reviewed derivation helper for
# maintainers who intentionally regenerate the derived BAM and CRAM artifacts.

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SOURCE_DIR="$ROOT_DIR/source"
BAM_DIR="$ROOT_DIR/bam/valid"
CRAM_DIR="$ROOT_DIR/cram/valid"

SOURCE_SAM="$SOURCE_DIR/tiny.valid.cram.explicit_ref.source.sam"
REFERENCE_FASTA="$SOURCE_DIR/tiny.ref.primary.fasta"
DERIVED_BAM="$BAM_DIR/tiny.valid.cram.explicit_ref.source.bam"
DERIVED_CRAM="$CRAM_DIR/tiny.valid.cram.explicit_ref.cram"

DRY_RUN=0
FORCE=0
SAMTOOLS_BIN="${SAMTOOLS_BIN:-samtools}"

usage() {
  cat <<'EOF'
Usage:
  tests/fixtures/source/generate_tiny_cram_fixture.sh [--dry-run] [--force]

Maintainer-oriented derivation helper for:
  * tests/fixtures/bam/valid/tiny.valid.cram.explicit_ref.source.bam
  * tests/fixtures/cram/valid/tiny.valid.cram.explicit_ref.cram

Options:
  --dry-run  Print the planned commands without executing them.
  --force    Allow overwriting existing derived BAM/CRAM outputs.
  -h, --help Show this help text.
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --dry-run)
      DRY_RUN=1
      shift
      ;;
    --force)
      FORCE=1
      shift
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "unknown argument: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

log() {
  printf '[generate_tiny_cram_fixture] %s\n' "$*"
}

run() {
  if [[ "$DRY_RUN" -eq 1 ]]; then
    printf '[dry-run] '
    printf '%q ' "$@"
    printf '\n'
  else
    "$@"
  fi
}

require_file() {
  local path="$1"
  if [[ ! -f "$path" ]]; then
    echo "required input file is missing: $path" >&2
    exit 1
  fi
}

require_tool() {
  local tool="$1"
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "required external tool was not found on PATH: $tool" >&2
    exit 1
  fi
}

check_output_path() {
  local path="$1"
  if [[ -e "$path" && "$FORCE" -ne 1 ]]; then
    echo "refusing to overwrite existing artifact without --force: $path" >&2
    exit 1
  fi
}

log "source-of-truth files remain the SAM and FASTA under tests/fixtures/source/"
log "derived artifacts will be written to deterministic BAM and CRAM paths"

require_file "$SOURCE_SAM"
require_file "$REFERENCE_FASTA"
require_tool "$SAMTOOLS_BIN"

run mkdir -p "$BAM_DIR" "$CRAM_DIR"

check_output_path "$DERIVED_BAM"
check_output_path "$DERIVED_CRAM"

log "deriving BAM from the source SAM"
run "$SAMTOOLS_BIN" view -b -o "$DERIVED_BAM" "$SOURCE_SAM"

log "deriving CRAM from the source BAM with the explicit reference FASTA"
run "$SAMTOOLS_BIN" view -C -T "$REFERENCE_FASTA" -o "$DERIVED_CRAM" "$DERIVED_BAM"

log "running lightweight container-level checks on the derived artifacts"
run "$SAMTOOLS_BIN" quickcheck "$DERIVED_BAM" "$DERIVED_CRAM"

log "printing concise headers for reviewer sanity checks"
run "$SAMTOOLS_BIN" view -H "$DERIVED_BAM"
run "$SAMTOOLS_BIN" view -H "$DERIVED_CRAM"

cat <<EOF

Derivation summary
  source SAM:       $SOURCE_SAM
  reference FASTA:  $REFERENCE_FASTA
  derived BAM:      $DERIVED_BAM
  derived CRAM:     $DERIVED_CRAM
  samtools binary:  $SAMTOOLS_BIN

Post-generation review expectations
  * Review the source SAM and FASTA first; they remain authoritative.
  * Confirm the derived BAM and CRAM are present and non-empty.
  * Confirm the CRAM decodes with the explicit FASTA.
  * Confirm strict-policy consume is expected to fail when the FASTA is withheld.
  * Treat byte-level CRAM drift as potentially tool-version-sensitive.
EOF
