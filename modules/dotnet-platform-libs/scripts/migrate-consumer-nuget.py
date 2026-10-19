#!/usr/bin/env python3
"""Check or apply the portable Platform NuGet consumer configuration."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import re
import sys
import xml.etree.ElementTree as ET


ROOT = Path(__file__).resolve().parents[1]
CANONICAL_CONFIG = ROOT / "eng" / "consumer" / "NuGet.config"
VERSION_PROPERTY = "$(PlatformPackageVersion)"
EXCLUDED_PARTS = {".git", "bin", "obj", "artifacts", "packages"}


def local_name(tag: str) -> str:
    return tag.rsplit("}", 1)[-1]


def project_files(root: Path) -> list[Path]:
    files = list(root.rglob("*.csproj")) + list(root.rglob("Directory.Packages.props"))
    return sorted({path for path in files if not EXCLUDED_PARTS.intersection(path.parts)})


def platform_elements(path: Path) -> tuple[ET.ElementTree, list[ET.Element]]:
    tree = ET.parse(path)
    elements = [
        element
        for element in tree.getroot().iter()
        if local_name(element.tag) in {"PackageReference", "PackageVersion"}
        and (element.get("Include") or "").startswith("Platform.")
    ]
    return tree, elements


def is_legacy_platform_project_reference(element: ET.Element) -> bool:
    if local_name(element.tag) != "ProjectReference":
        return False
    include = (element.get("Include") or "").replace("\\", "/")
    return "PlatformConsumerSourceRoot" in include or "/src/Platform." in include


def validate_legacy_groups(path: Path, tree: ET.ElementTree) -> None:
    for group in list(tree.getroot()):
        condition = group.get("Condition") or ""
        if local_name(group.tag) != "ItemGroup" or "PlatformAsSource" not in condition:
            continue
        if "==" in condition and "'true'" in condition:
            children = list(group)
            legacy = [item for item in children if is_legacy_platform_project_reference(item)]
            if legacy and len(legacy) != len(children):
                raise ValueError(
                    f"mixed legacy source ItemGroup cannot be migrated atomically in {path}"
                )


def exact_literal(version: str) -> bool:
    return bool(re.fullmatch(r"[0-9]+\.[0-9]+\.[0-9]+(?:-[0-9A-Za-z.-]+)?", version))


def inspect(root: Path) -> tuple[list[tuple[Path, ET.ElementTree, list[ET.Element]]], str]:
    parsed = []
    literals: set[str] = set()
    references = 0
    for path in project_files(root):
        tree, elements = platform_elements(path)
        if not elements:
            continue
        validate_legacy_groups(path, tree)
        parsed.append((path, tree, elements))
        references += len(elements)
        for element in elements:
            version = element.get("Version")
            if version and version != VERSION_PROPERTY:
                if not exact_literal(version):
                    raise ValueError(f"non-exact Platform.* version {version!r} in {path}")
                literals.add(version)

    for props in root.glob("Directory.*.props"):
        try:
            tree = ET.parse(props)
        except ET.ParseError:
            continue
        for element in tree.getroot().iter():
            if local_name(element.tag) == "PlatformPackageVersion" and element.text:
                value = element.text.strip()
                if not exact_literal(value):
                    raise ValueError(f"non-exact PlatformPackageVersion {value!r} in {props}")
                literals.add(value)

    if references == 0:
        raise ValueError(f"no Platform.* package references found under {root}")
    if len(literals) > 1:
        raise ValueError("multiple Platform.* versions found: " + ", ".join(sorted(literals)))
    if not literals:
        raise ValueError("Platform.* references have no resolvable exact version")
    return parsed, next(iter(literals))


def atomic_tree(tree: ET.ElementTree, path: Path) -> None:
    ET.indent(tree, space="  ")
    temporary = path.with_name(path.name + ".tmp")
    tree.write(temporary, encoding="utf-8", xml_declaration=True)
    with temporary.open("ab") as stream:
        stream.write(b"\n")
    os.replace(temporary, path)


def ensure_version_property(root: Path, version: str) -> None:
    central = root / "Directory.Packages.props"
    target = central if central.exists() else root / "Directory.Build.props"
    tree = ET.parse(target) if target.exists() else ET.ElementTree(ET.Element("Project"))
    project = tree.getroot()
    existing = [item for item in project.iter() if local_name(item.tag) == "PlatformPackageVersion"]
    if existing:
        element = existing[0]
        element.text = version
        element.set("Condition", "'$(PlatformPackageVersion)' == ''")
        for duplicate in existing[1:]:
            for parent in project.iter():
                if duplicate in list(parent):
                    parent.remove(duplicate)
                    break
    else:
        group = ET.Element("PropertyGroup")
        element = ET.SubElement(
            group,
            "PlatformPackageVersion",
            {"Condition": "'$(PlatformPackageVersion)' == ''"},
        )
        element.text = version
        project.insert(0, group)
    atomic_tree(tree, target)


def contains_legacy_source_switch(tree: ET.ElementTree) -> bool:
    for element in tree.getroot().iter():
        if "PlatformAsSource" in (element.get("Condition") or ""):
            return True
        if is_legacy_platform_project_reference(element):
            return True
    return False


def is_conforming(
    root: Path,
    parsed: list[tuple[Path, ET.ElementTree, list[ET.Element]]],
    version: str,
) -> bool:
    config = root / "NuGet.config"
    if not config.exists() or config.read_bytes() != CANONICAL_CONFIG.read_bytes():
        return False
    for _, tree, elements in parsed:
        if any(element.get("Version") != VERSION_PROPERTY for element in elements):
            return False
        if contains_legacy_source_switch(tree):
            return False
    props = root / "Directory.Packages.props"
    props = props if props.exists() else root / "Directory.Build.props"
    if not props.exists():
        return False
    tree = ET.parse(props)
    matches = [
        item
        for item in tree.getroot().iter()
        if local_name(item.tag) == "PlatformPackageVersion"
        and item.get("Condition") == "'$(PlatformPackageVersion)' == ''"
        and (item.text or "").strip() == version
    ]
    if len(matches) != 1:
        return False
    for props_path in root.glob("Directory.*.props"):
        props_tree = ET.parse(props_path)
        if any(
            local_name(item.tag) in {"PlatformAsSource", "PlatformConsumerSourceRoot"}
            for item in props_tree.getroot().iter()
        ):
            return False
    return True


def remove_legacy_source_switch(tree: ET.ElementTree) -> bool:
    project = tree.getroot()
    changed = False
    for group in list(project):
        condition = group.get("Condition") or ""
        if local_name(group.tag) != "ItemGroup" or "PlatformAsSource" not in condition:
            continue
        if "==" in condition and "'true'" in condition:
            project.remove(group)
            changed = True
        elif "!=" in condition and "'true'" in condition:
            group.attrib.pop("Condition", None)
            changed = True
    return changed


def remove_legacy_properties(root: Path) -> None:
    for path in root.glob("Directory.*.props"):
        tree = ET.parse(path)
        changed = False
        for parent in tree.getroot().iter():
            for child in list(parent):
                if local_name(child.tag) in {"PlatformAsSource", "PlatformConsumerSourceRoot"}:
                    parent.remove(child)
                    changed = True
        if changed:
            atomic_tree(tree, path)


def apply(root: Path, parsed: list[tuple[Path, ET.ElementTree, list[ET.Element]]], version: str) -> None:
    for path, tree, elements in parsed:
        changed = remove_legacy_source_switch(tree)
        for element in elements:
            if element.get("Version") != VERSION_PROPERTY:
                element.set("Version", VERSION_PROPERTY)
                changed = True
        if changed:
            atomic_tree(tree, path)
    remove_legacy_properties(root)
    ensure_version_property(root, version)
    temporary = root / "NuGet.config.tmp"
    temporary.write_bytes(CANONICAL_CONFIG.read_bytes())
    os.replace(temporary, root / "NuGet.config")


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("root", type=Path)
    parser.add_argument("--apply", action="store_true")
    args = parser.parse_args()
    root = args.root.resolve()
    try:
        if not root.is_dir():
            raise ValueError(f"consumer root does not exist: {root}")
        parsed, version = inspect(root)
        if is_conforming(root, parsed, version):
            print(f"PASS: {root} already conforms")
            return 0
        if not args.apply:
            print(f"DRIFT: {root} requires portable Platform NuGet migration", file=sys.stderr)
            return 1
        apply(root, parsed, version)
        reparsed, _ = inspect(root)
        if not is_conforming(root, reparsed, version):
            raise ValueError("migration completed but conformance check still fails")
        print(f"MIGRATED: {root} uses PlatformPackageVersion={version}")
        return 0
    except (ValueError, OSError, ET.ParseError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
