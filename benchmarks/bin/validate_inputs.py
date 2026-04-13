#!/usr/bin/env python3
"""Validate a Bamana benchmark input manifest."""

from __future__ import annotations

import argparse
import json
import os
import sys
from pathlib import Path


ALLOWED_TYPES = {"mapped_bam", "unmapped_bam", "fastq_gz"}
ALLOWED_SCENARIOS = {
    "mapped_bam_pipeline",
    "unmapped_bam_pipeline",
    "fastq_consume_pipeline",
    "subsample_only",
}
ALLOWED_STAGING_MODES = {"direct", "copy", "hardlink", "symlink", "scratch_copy"}


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate Bamana benchmark input manifests.")
    parser.add_argument("--manifest", required=True, help="Path to the benchmark input manifest JSON.")
    parser.add_argument(
        "--skip-file-checks",
        action="store_true",
        help="Validate manifest structure without checking that local source paths exist.",
    )
    args = parser.parse_args()

    manifest_path = Path(args.manifest)
    try:
        manifest = json.loads(manifest_path.read_text())
    except FileNotFoundError:
        print(f"manifest not found: {manifest_path}", file=sys.stderr)
        return 2
    except json.JSONDecodeError as exc:
        print(f"manifest is not valid JSON: {exc}", file=sys.stderr)
        return 2

    entries = manifest.get("inputs", manifest.get("entries"))
    if not isinstance(entries, list) or not entries:
        print("manifest must contain a non-empty 'inputs' array", file=sys.stderr)
        return 2

    seen_ids: set[str] = set()
    failures: list[str] = []

    for index, entry in enumerate(entries, start=1):
        label = f"entry[{index}]"
        entry_id = entry.get("id")
        if not isinstance(entry_id, str) or not entry_id.strip():
            failures.append(f"{label}: missing non-empty id")
            continue
        if entry_id in seen_ids:
            failures.append(f"{label}: duplicate id '{entry_id}'")
        seen_ids.add(entry_id)

        entry_type = entry.get("type")
        if entry_type not in ALLOWED_TYPES:
            failures.append(f"{entry_id}: unsupported type '{entry_type}'")

        compression = entry.get("compression")
        if compression not in {"bgzf", "gzip", "plain", "unknown"}:
            failures.append(f"{entry_id}: unsupported compression '{compression}'")

        mapped_state = entry.get("mapped_state")
        if mapped_state not in {"mapped", "unmapped", "unknown"}:
            failures.append(f"{entry_id}: unsupported mapped_state '{mapped_state}'")

        expected_sort_order = entry.get("expected_sort_order")
        if expected_sort_order not in {"coordinate", "queryname", "unsorted", "unknown", "not_applicable"}:
            failures.append(f"{entry_id}: unsupported expected_sort_order '{expected_sort_order}'")

        path = entry.get("path")
        if not isinstance(path, str) or not path.strip():
            failures.append(f"{entry_id}: missing path")
        elif not args.skip_file_checks:
            source = Path(path)
            if not source.is_absolute():
                failures.append(f"{entry_id}: source path must be absolute")
            elif not source.exists():
                failures.append(f"{entry_id}: source path does not exist: {source}")
            elif not os.access(source, os.R_OK):
                failures.append(f"{entry_id}: source path is not readable: {source}")

        has_index = entry.get("has_index")
        index_path = entry.get("index_path")
        if has_index is True and (not isinstance(index_path, str) or not index_path.strip()):
            failures.append(f"{entry_id}: has_index=true requires a non-empty index_path")

        scenarios = entry.get("allowed_benchmark_scenarios")
        if not isinstance(scenarios, list) or not scenarios:
            failures.append(f"{entry_id}: allowed_benchmark_scenarios must be a non-empty array")
        else:
            unsupported = [scenario for scenario in scenarios if scenario not in ALLOWED_SCENARIOS]
            if unsupported:
                failures.append(f"{entry_id}: unsupported scenarios: {', '.join(unsupported)}")

        reference_context = entry.get("reference_context")
        if reference_context is not None and not isinstance(reference_context, dict):
            failures.append(f"{entry_id}: reference_context must be an object or null")

        staging_policy = entry.get("staging_policy")
        if not isinstance(staging_policy, dict):
            failures.append(f"{entry_id}: staging_policy must be an object")
        else:
            mode = staging_policy.get("mode")
            if mode not in ALLOWED_STAGING_MODES:
                failures.append(f"{entry_id}: unsupported staging mode '{mode}'")

    if failures:
        print("benchmark input manifest validation failed:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1

    summary = {
        "manifest": str(manifest_path),
        "inputs": len(entries),
        "ids": [entry["id"] for entry in entries],
    }
    print(json.dumps(summary, indent=2))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
