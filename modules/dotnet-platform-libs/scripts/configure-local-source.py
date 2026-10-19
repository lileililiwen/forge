#!/usr/bin/env python3
"""Configure the optional Platform local feed in the current user's NuGet config."""

from __future__ import annotations

import argparse
import os
from pathlib import Path
import subprocess
import sys
import xml.etree.ElementTree as ET


SOURCE_NAME = "platform-local"
PATTERN = "Platform.*"


def default_config() -> Path:
    if os.name == "nt":
        appdata = os.environ.get("APPDATA")
        if not appdata:
            raise ValueError("APPDATA is unset; pass --configfile")
        return Path(appdata) / "NuGet" / "NuGet.Config"
    return Path.home() / ".nuget" / "NuGet" / "NuGet.Config"


def load_or_create(path: Path) -> ET.ElementTree:
    if path.exists():
        return ET.parse(path)
    path.parent.mkdir(parents=True, exist_ok=True)
    return ET.ElementTree(ET.Element("configuration"))


def write_tree(tree: ET.ElementTree, path: Path) -> None:
    ET.indent(tree, space="  ")
    temporary = path.with_name(path.name + ".tmp")
    tree.write(temporary, encoding="utf-8", xml_declaration=True)
    with temporary.open("ab") as stream:
        stream.write(b"\n")
    os.replace(temporary, path)


def has_source(path: Path) -> bool:
    if not path.exists():
        return False
    root = ET.parse(path).getroot()
    sources = root.find("packageSources")
    return sources is not None and any(
        item.tag == "add" and item.get("key") == SOURCE_NAME for item in sources
    )


def run_dotnet(action: str, config: Path, feed: Path | None = None) -> None:
    command = ["dotnet", "nuget", action, "source"]
    if action == "add":
        assert feed is not None
        command.extend([str(feed), "--name", SOURCE_NAME])
    elif action == "update":
        assert feed is not None
        command.extend([SOURCE_NAME, "--source", str(feed)])
    else:
        command.append(SOURCE_NAME)
    command.extend(["--configfile", str(config)])
    result = subprocess.run(command, text=True, capture_output=True, check=False)
    if result.returncode != 0:
        message = result.stderr.strip() or result.stdout.strip()
        raise ValueError(f"{' '.join(command[:4])} failed: {message}")


def ensure_mapping(config: Path, present: bool) -> None:
    tree = load_or_create(config)
    root = tree.getroot()
    mappings = root.find("packageSourceMapping")
    if mappings is None and present:
        mappings = ET.SubElement(root, "packageSourceMapping")
    if mappings is None:
        return
    for source in list(mappings.findall("packageSource")):
        if source.get("key") == SOURCE_NAME:
            mappings.remove(source)
    if present:
        source = ET.SubElement(mappings, "packageSource", {"key": SOURCE_NAME})
        ET.SubElement(source, "package", {"pattern": PATTERN})
    elif len(mappings) == 0:
        root.remove(mappings)
    write_tree(tree, config)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--feed", type=Path)
    parser.add_argument("--remove", action="store_true")
    parser.add_argument("--configfile", type=Path)
    args = parser.parse_args()
    try:
        config = (args.configfile or default_config()).resolve()
        if args.remove:
            if has_source(config):
                run_dotnet("remove", config)
            ensure_mapping(config, False)
            print(f"Removed {SOURCE_NAME} from {config}")
            return 0
        if args.feed is None:
            raise ValueError("--feed is required unless --remove is used")
        feed = args.feed.resolve()
        if not feed.is_dir():
            raise ValueError(f"local feed directory does not exist: {feed}")
        config.parent.mkdir(parents=True, exist_ok=True)
        if not config.exists():
            write_tree(load_or_create(config), config)
        run_dotnet("update" if has_source(config) else "add", config, feed)
        ensure_mapping(config, True)
        print(f"Configured {SOURCE_NAME} in {config}")
        return 0
    except (ValueError, OSError, ET.ParseError) as error:
        print(f"ERROR: {error}", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
