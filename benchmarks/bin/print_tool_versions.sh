#!/usr/bin/env bash
set -euo pipefail

print_version() {
  local name="$1"
  shift
  local value
  value="$("$@" | head -n 1 || true)"
  printf "%s\t%s\n" "$name" "$value"
}

print_version nextflow nextflow -version
print_version java bash -lc "java -version 2>&1"
print_version samtools samtools --version
print_version sambamba sambamba --version
print_version seqtk bash -lc "seqtk 2>&1"
print_version rasusa rasusa --version
print_version fastcat fastcat --version
print_version R R --version
