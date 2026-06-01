#!/usr/bin/env python3
"""Check benchmark schema inventory and stability metadata."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path


def load_json(path: Path) -> object:
    with path.open("r", encoding="utf-8") as handle:
        return json.load(handle)


def pointer_get(document: object, pointer: str) -> object:
    if not pointer.startswith("/"):
        raise ValueError(f"JSON pointer must start with '/': {pointer}")
    current = document
    for raw_part in pointer.split("/")[1:]:
        part = raw_part.replace("~1", "/").replace("~0", "~")
        if isinstance(current, dict):
            if part not in current:
                raise KeyError(pointer)
            current = current[part]
        elif isinstance(current, list):
            try:
                current = current[int(part)]
            except (ValueError, IndexError) as error:
                raise KeyError(pointer) from error
        else:
            raise KeyError(pointer)
    return current


def benchmark_schema_files(repo_root: Path) -> set[str]:
    return {
        path.relative_to(repo_root).as_posix()
        for path in repo_root.joinpath("benchmarks").rglob("*.schema.json")
    }


def validate_manifest(repo_root: Path, manifest_path: Path) -> tuple[list[str], list[str]]:
    failures: list[str] = []
    checked: list[str] = []
    manifest = load_json(manifest_path)
    if not isinstance(manifest, dict):
        return ["schema stability manifest must be a JSON object"], checked

    entries = manifest.get("schemas")
    if not isinstance(entries, list) or not entries:
        return ["schema stability manifest must contain a non-empty schemas array"], checked

    manifest_paths: set[str] = set()
    seen_ids: set[str] = set()

    for index, entry in enumerate(entries):
        if not isinstance(entry, dict):
            failures.append(f"schemas[{index}] must be an object")
            continue
        rel_path = entry.get("path")
        expected_id = entry.get("id")
        version_pointer = entry.get("version_pointer")
        expected_version = entry.get("version")
        required_pointers = entry.get("required_pointers")

        if not isinstance(rel_path, str) or not rel_path:
            failures.append(f"schemas[{index}] missing path")
            continue
        manifest_paths.add(rel_path)
        checked.append(rel_path)
        path = repo_root.joinpath(rel_path)
        if not path.is_file():
            failures.append(f"{rel_path}: schema file is missing")
            continue

        try:
            schema = load_json(path)
        except json.JSONDecodeError as error:
            failures.append(f"{rel_path}: JSON parse failed: {error}")
            continue
        if not isinstance(schema, dict):
            failures.append(f"{rel_path}: schema root must be an object")
            continue

        actual_schema = schema.get("$schema")
        if actual_schema != "https://json-schema.org/draft/2020-12/schema":
            failures.append(f"{rel_path}: unexpected $schema {actual_schema!r}")
        actual_id = schema.get("$id")
        if actual_id != expected_id:
            failures.append(f"{rel_path}: expected $id {expected_id!r}, got {actual_id!r}")
        if isinstance(actual_id, str):
            if actual_id in seen_ids:
                failures.append(f"{rel_path}: duplicate $id {actual_id!r}")
            seen_ids.add(actual_id)

        if not isinstance(version_pointer, str) or not version_pointer:
            failures.append(f"{rel_path}: version_pointer must be non-empty text")
        else:
            try:
                actual_version = pointer_get(schema, version_pointer)
            except KeyError:
                failures.append(f"{rel_path}: missing version pointer {version_pointer}")
            else:
                if actual_version != expected_version:
                    failures.append(
                        f"{rel_path}: expected version {expected_version!r} at "
                        f"{version_pointer}, got {actual_version!r}"
                    )

        if not isinstance(required_pointers, list) or not required_pointers:
            failures.append(f"{rel_path}: required_pointers must be a non-empty array")
        else:
            for pointer in required_pointers:
                if not isinstance(pointer, str):
                    failures.append(f"{rel_path}: required pointer {pointer!r} is not text")
                    continue
                try:
                    pointer_get(schema, pointer)
                except KeyError:
                    failures.append(f"{rel_path}: missing required pointer {pointer}")

    actual_paths = benchmark_schema_files(repo_root)
    missing_from_manifest = sorted(actual_paths - manifest_paths)
    stale_manifest_paths = sorted(manifest_paths - actual_paths)
    for rel_path in missing_from_manifest:
        failures.append(f"{rel_path}: benchmark schema is not listed in stability manifest")
    for rel_path in stale_manifest_paths:
        failures.append(f"{rel_path}: manifest lists a schema that does not exist")

    return failures, checked


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--manifest",
        default="benchmarks/schema_stability_manifest.json",
        help="Path to the benchmark schema stability manifest.",
    )
    parser.add_argument(
        "--repo-root",
        default=".",
        help="Repository root used to resolve manifest paths.",
    )
    args = parser.parse_args()

    repo_root = Path(args.repo_root).resolve()
    manifest_path = Path(args.manifest)
    if not manifest_path.is_absolute():
        manifest_path = repo_root.joinpath(manifest_path)

    failures, checked = validate_manifest(repo_root, manifest_path)
    if failures:
        print("benchmark schema stability check failed:", file=sys.stderr)
        for failure in failures:
            print(f"  - {failure}", file=sys.stderr)
        return 1

    print(
        json.dumps(
            {
                "manifest": str(manifest_path.relative_to(repo_root)),
                "schemas": len(checked),
                "paths": checked,
            },
            indent=2,
        )
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
