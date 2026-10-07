from __future__ import annotations

import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
import zipfile
import xml.etree.ElementTree as ET


ROOT = Path(__file__).resolve().parents[2]
MIGRATE = ROOT / "scripts" / "migrate-consumer-nuget.py"
LOCAL_SOURCE = ROOT / "scripts" / "configure-local-source.py"
PREPARE_FEED = ROOT / "scripts" / "prepare-static-feed.py"


def run_script(script: Path, *args: str) -> subprocess.CompletedProcess[str]:
    return subprocess.run(
        [sys.executable, str(script), *args],
        cwd=ROOT,
        text=True,
        capture_output=True,
        check=False,
    )


def write_package(path: Path, package_id: str, version: str, payload: bytes) -> None:
    nuspec = f"""<?xml version="1.0"?>
<package><metadata><id>{package_id}</id><version>{version}</version></metadata></package>
"""
    with zipfile.ZipFile(path, "w") as archive:
        archive.writestr(f"{package_id}.nuspec", nuspec)
        archive.writestr("lib/net10.0/content.dll", payload)


class ConsumerMigrationTests(unittest.TestCase):
    def test_apply_migrates_inline_version(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            project = root / "App.csproj"
            project.write_text(
                """<Project Sdk="Microsoft.NET.Sdk"><ItemGroup>
<PackageReference Include="Platform.Core" Version="0.1.0-dev.7" />
<PackageReference Include="xunit" Version="2.9.2" />
</ItemGroup></Project>\n""",
                encoding="utf-8",
            )

            applied = run_script(MIGRATE, str(root), "--apply")

            self.assertEqual(0, applied.returncode, applied.stdout + applied.stderr)
            content = project.read_text(encoding="utf-8")
            self.assertIn('Include="Platform.Core" Version="$(PlatformPackageVersion)"', content)
            self.assertIn('Include="xunit" Version="2.9.2"', content)
            props = (root / "Directory.Build.props").read_text(encoding="utf-8")
            self.assertIn(">0.1.0-dev.7</PlatformPackageVersion>", props)

    def test_apply_removes_legacy_source_switch_and_keeps_package_references(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Directory.Build.props").write_text(
                """<Project><PropertyGroup>
<PlatformAsSource Condition="'$(PlatformAsSource)' == ''">false</PlatformAsSource>
<PlatformConsumerSourceRoot>/home/dev/dotnet-platform-libs</PlatformConsumerSourceRoot>
<PlatformPackageVersion Condition="'$(PlatformPackageVersion)' == ''">0.1.0</PlatformPackageVersion>
</PropertyGroup></Project>\n""",
                encoding="utf-8",
            )
            project = root / "App.csproj"
            project.write_text(
                """<Project Sdk="Microsoft.NET.Sdk">
<ItemGroup Condition="'$(PlatformAsSource)' == 'true'">
  <ProjectReference Include="$(PlatformConsumerSourceRoot)/src/Platform.Core/Platform.Core.csproj" />
</ItemGroup>
<ItemGroup Condition="'$(PlatformAsSource)' != 'true'">
  <PackageReference Include="Platform.Core" Version="$(PlatformPackageVersion)" />
</ItemGroup>
</Project>\n""",
                encoding="utf-8",
            )

            applied = run_script(MIGRATE, str(root), "--apply")

            self.assertEqual(0, applied.returncode, applied.stdout + applied.stderr)
            migrated_project = project.read_text(encoding="utf-8")
            self.assertNotIn("PlatformAsSource", migrated_project)
            self.assertNotIn("PlatformConsumerSourceRoot", migrated_project)
            self.assertNotIn("ProjectReference", migrated_project)
            self.assertIn('PackageReference Include="Platform.Core"', migrated_project)
            migrated_props = (root / "Directory.Build.props").read_text(encoding="utf-8")
            self.assertNotIn("PlatformAsSource", migrated_props)
            self.assertNotIn("PlatformConsumerSourceRoot", migrated_props)

    def test_apply_migrates_central_versions_and_is_idempotent(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "NuGet.config").write_text(
                """<configuration><packageSources><clear />
<add key="local" value="/home/dev/feed" />
<add key="github" value="https://nuget.pkg.github.com/example/index.json" />
</packageSources></configuration>\n""",
                encoding="utf-8",
            )
            props = root / "Directory.Packages.props"
            props.write_text(
                """<Project><PropertyGroup><ManagePackageVersionsCentrally>true</ManagePackageVersionsCentrally></PropertyGroup>
<ItemGroup><PackageVersion Include="Platform.Core" Version="0.1.0" />
<PackageVersion Include="Platform.AspNetCore" Version="0.1.0" />
<PackageVersion Include="xunit" Version="2.9.2" /></ItemGroup></Project>\n""",
                encoding="utf-8",
            )

            check = run_script(MIGRATE, str(root))
            self.assertEqual(1, check.returncode, check.stdout + check.stderr)

            applied = run_script(MIGRATE, str(root), "--apply")
            self.assertEqual(0, applied.returncode, applied.stdout + applied.stderr)
            nuget = (root / "NuGet.config").read_text(encoding="utf-8")
            self.assertIn("https://lileililiwen.github.io/dotnet-platform-libs/nuget/index.json", nuget)
            self.assertIn('<package pattern="Platform.*" />', nuget)
            self.assertNotIn("/home/dev", nuget)
            self.assertNotIn("nuget.pkg.github.com", nuget)
            migrated_props = props.read_text(encoding="utf-8")
            self.assertIn("<PlatformPackageVersion Condition=", migrated_props)
            migrated_xml = ET.fromstring(migrated_props)
            platform_versions = [
                element.get("Version")
                for element in migrated_xml.iter()
                if element.tag == "PackageVersion"
                and (element.get("Include") or "").startswith("Platform.")
            ]
            self.assertEqual(
                ["$(PlatformPackageVersion)", "$(PlatformPackageVersion)"],
                platform_versions,
            )
            self.assertIn('Version="2.9.2"', migrated_props)

            before = {path: path.read_bytes() for path in (root / "NuGet.config", props)}
            repeated = run_script(MIGRATE, str(root), "--apply")
            self.assertEqual(0, repeated.returncode, repeated.stdout + repeated.stderr)
            self.assertEqual(before, {path: path.read_bytes() for path in before})

    def test_ambiguous_versions_refuse_every_write(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            project = root / "App.csproj"
            project.write_text(
                """<Project Sdk="Microsoft.NET.Sdk"><ItemGroup>
<PackageReference Include="Platform.Core" Version="0.1.0" />
<PackageReference Include="Platform.AspNetCore" Version="0.2.0" />
</ItemGroup></Project>\n""",
                encoding="utf-8",
            )
            before = project.read_bytes()

            result = run_script(MIGRATE, str(root), "--apply")

            self.assertEqual(2, result.returncode, result.stdout + result.stderr)
            self.assertIn("multiple Platform.* versions", result.stderr)
            self.assertEqual(before, project.read_bytes())
            self.assertFalse((root / "NuGet.config").exists())


class LocalSourceTests(unittest.TestCase):
    def test_add_and_remove_are_user_config_only_and_idempotent(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            feed = root / "feed"
            feed.mkdir()
            config = root / "NuGet.Config"

            first = run_script(
                LOCAL_SOURCE,
                "--feed",
                str(feed),
                "--configfile",
                str(config),
            )
            self.assertEqual(0, first.returncode, first.stdout + first.stderr)
            second = run_script(
                LOCAL_SOURCE,
                "--feed",
                str(feed),
                "--configfile",
                str(config),
            )
            self.assertEqual(0, second.returncode, second.stdout + second.stderr)
            content = config.read_text(encoding="utf-8")
            config_xml = ET.fromstring(content)
            self.assertEqual(
                1,
                len(config_xml.findall("./packageSources/add[@key='platform-local']")),
            )
            self.assertEqual(
                1,
                len(
                    config_xml.findall(
                        "./packageSourceMapping/packageSource[@key='platform-local']"
                    )
                ),
            )
            self.assertIn('<package pattern="Platform.*" />', content)

            removed = run_script(
                LOCAL_SOURCE,
                "--remove",
                "--configfile",
                str(config),
            )
            self.assertEqual(0, removed.returncode, removed.stdout + removed.stderr)
            content = config.read_text(encoding="utf-8")
            self.assertNotIn("platform-local", content)

    def test_repository_mapping_merges_with_user_local_source_for_restore(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            home = root / "home"
            user_config = home / ".nuget" / "NuGet" / "NuGet.Config"
            local_feed = root / "local-feed"
            remote_feed = root / "remote-feed"
            consumer = root / "consumer"
            local_feed.mkdir()
            remote_feed.mkdir()
            consumer.mkdir()
            package = local_feed / "Platform.Core.0.1.0-dev.merge.nupkg"
            write_package(package, "Platform.Core", "0.1.0-dev.merge", b"fixture")

            configured = run_script(
                LOCAL_SOURCE,
                "--feed",
                str(local_feed),
                "--configfile",
                str(user_config),
            )
            self.assertEqual(0, configured.returncode, configured.stdout + configured.stderr)
            canonical = (ROOT / "eng" / "consumer" / "NuGet.config").read_text(
                encoding="utf-8"
            )
            canonical = canonical.replace(
                "https://lileililiwen.github.io/dotnet-platform-libs/nuget/index.json",
                str(remote_feed),
            )
            canonical = canonical.replace(
                '<package pattern="Platform.*" />',
                '<package pattern="Platform.RemoteOnly.*" />',
            )
            (consumer / "NuGet.config").write_text(canonical, encoding="utf-8")
            (consumer / "App.csproj").write_text(
                """<Project Sdk="Microsoft.NET.Sdk"><PropertyGroup>
<TargetFramework>net10.0</TargetFramework>
</PropertyGroup><ItemGroup>
<PackageReference Include="Platform.Core" Version="0.1.0-dev.merge" />
</ItemGroup></Project>\n""",
                encoding="utf-8",
            )
            environment = os.environ.copy()
            environment["HOME"] = str(home)
            environment["NUGET_PACKAGES"] = str(root / "packages")

            restored = subprocess.run(
                ["dotnet", "restore", "App.csproj", "--nologo"],
                cwd=consumer,
                env=environment,
                text=True,
                capture_output=True,
                check=False,
            )

            self.assertEqual(0, restored.returncode, restored.stdout + restored.stderr)
            tracked_config = (consumer / "NuGet.config").read_text(encoding="utf-8")
            self.assertNotIn(str(local_feed), tracked_config)
            self.assertIn("platform-local", user_config.read_text(encoding="utf-8"))


class StaticFeedPreparationTests(unittest.TestCase):
    def test_prepare_records_immutable_packages_and_refuses_changed_bytes(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            packages = root / "packages"
            feed = root / "site" / "nuget"
            packages.mkdir()
            package = packages / "Platform.Core.0.1.0.nupkg"
            write_package(package, "Platform.Core", "0.1.0", b"first")

            first = run_script(
                PREPARE_FEED,
                "--packages",
                str(packages),
                "--feed-root",
                str(feed),
                "--base-uri",
                "https://packages.example.test/nuget/",
            )
            self.assertEqual(0, first.returncode, first.stdout + first.stderr)
            manifest_path = feed / "platform-feed-manifest.json"
            manifest = json.loads(manifest_path.read_text(encoding="utf-8"))
            self.assertEqual(
                hashlib.sha256(package.read_bytes()).hexdigest(),
                manifest["packages"]["platform.core/0.1.0"]["sha256"],
            )
            self.assertEqual("https://packages.example.test/nuget/", manifest["baseUri"])

            write_package(package, "Platform.Core", "0.1.0", b"changed")
            before = manifest_path.read_bytes()
            conflict = run_script(
                PREPARE_FEED,
                "--packages",
                str(packages),
                "--feed-root",
                str(feed),
                "--base-uri",
                "https://packages.example.test/nuget/",
            )
            self.assertEqual(2, conflict.returncode, conflict.stdout + conflict.stderr)
            self.assertIn("immutable package conflict", conflict.stderr)
            self.assertEqual(before, manifest_path.read_bytes())


if __name__ == "__main__":
    unittest.main()
