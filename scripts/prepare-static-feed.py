#!/usr/bin/env python3
"""Prepare immutable metadata and Sleet configuration for the public feed."""

from __future__ import annotations

import argparse
import hashlib
import json
from pathlib import Path
import os
import sys
import zipfile
import xml.etree.ElementTree as ET


def package_identity(path: Path) -> tuple[str, str]:
    try:
        with zipfile.ZipFile(path) as archive:
            nuspecs = [name for name in archive.namelist() if name.lower().endswith(".nuspec")]
            if len(nuspecs) != 1:
                raise ValueError(f"expected one nuspec, found {len(nuspecs)}")
            root = ET.fromstring(archive.read(nuspecs[0]))
    except (OSError, zipfile.BadZipFile, ET.ParseError, ValueError) as error:
        raise ValueError(f"invalid package {path}: {error}") from error

    values: dict[str, str] = {}
    for element in root.iter():
        name = element.tag.rsplit("}", 1)[-1]
        if name in {"id", "version"} and element.text:
            values[name] = element.text.strip()
    if not values.get("id") or not values.get("version"):
        raise ValueError(f"invalid package {path}: nuspec id/version is missing")
    if not values["id"].startswith("Platform."):
        raise ValueError(f"invalid package {path}: package id must start with Platform.")
    return values["id"], values["version"]


def atomic_json(path: Path, value: object) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    temporary = path.with_name(path.name + ".tmp")
    temporary.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    os.replace(temporary, path)


def prepare(packages: Path, feed_root: Path, base_uri: str, sleet_config: Path) -> None:
    if not packages.is_dir():
        raise ValueError(f"package directory does not exist: {packages}")
    if not base_uri.startswith("https://") or not base_uri.endswith("/"):
        raise ValueError("base URI must be an https URL ending in /")
    package_paths = sorted(packages.glob("*.nupkg"))
    if not package_paths:
        raise ValueError(f"no .nupkg files found in {packages}")

    manifest_path = feed_root / "platform-feed-manifest.json"
    prior = {"schema": 1, "baseUri": base_uri, "packages": {}}
    if manifest_path.exists():
        prior = json.loads(manifest_path.read_text(encoding="utf-8"))
        if prior.get("baseUri") != base_uri:
            raise ValueError(
                f"feed base URI changed from {prior.get('baseUri')!r} to {base_uri!r}; rebuild explicitly"
            )

    updated = dict(prior.get("packages", {}))
    for path in package_paths:
        package_id, version = package_identity(path)
        digest = hashlib.sha256(path.read_bytes()).hexdigest()
        key = f"{package_id.lower()}/{version.lower()}"
        existing = updated.get(key)
        if existing and existing.get("sha256") != digest:
            raise ValueError(
                f"immutable package conflict for {package_id} {version}: published bytes differ"
            )
        updated[key] = {
            "id": package_id,
            "version": version,
            "sha256": digest,
            "file": path.name,
        }

    feed_root.mkdir(parents=True, exist_ok=True)
    atomic_json(
        manifest_path,
        {"schema": 1, "baseUri": base_uri, "packages": updated},
    )
    atomic_json(
        sleet_config,
        {
            "username": "dotnet-platform-libs release workflow",
            "useremail": "noreply@github.com",
            "sources": [
                {
                    "name": "platform-pages",
                    "type": "local",
                    "path": str(feed_root.resolve()),
                    "baseURI": base_uri,
                }
            ],
        },
    )


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--packages", required=True, type=Path)
    parser.add_argument("--feed-root", required=True, type=Path)
    parser.add_argument("--base-uri", required=True)
    parser.add_argument("--sleet-config", type=Path)
    args = parser.parse_args()
    config = args.sleet_config or args.feed_root.parent / "sleet.json"
    try:
        prepare(args.packages, args.feed_root, args.base_uri, config)
    except (ValueError, OSError, json.JSONDecodeError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 2
    print(f"Prepared {args.feed_root} for {args.base_uri}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
