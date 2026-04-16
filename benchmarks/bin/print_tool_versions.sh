#!/usr/bin/env bash
set -euo pipefail

print_version() {
  local name="$1"
  shift
  local value
  value="$("$@" 2>&1 | head -n 1 || true)"
  value="$(printf '%s' "$value" | sed -E $'s/\x1B\\[[0-9;]*[[:alpha:]]//g' | tr -d '\033\r')"
  printf "%s\t%s\n" "$name" "$value"
}

print_version nextflow nextflow -version
print_version java bash -lc "java -version 2>&1"
print_version samtools samtools --version
print_version sambamba sambamba --version
print_version seqtk bash -lc "seqtk 2>&1"
print_version rasusa rasusa --version
print_version fastcat fastcat fastq --version
print_version R R --version
