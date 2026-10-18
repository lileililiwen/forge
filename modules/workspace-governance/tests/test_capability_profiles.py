"""Cross-cutting shared capabilities are declarable in every profile.

Covers the scenarios in
`openspec/changes/admit-cross-cutting-capabilities/specs/platform-capability-manifest/spec.md`:

- CC-001 the four cross-cutting shared ids are in every profile that declares a
  `supported_capabilities` list
- CC-002 a lawful cross-cutting declaration emits no `CAPABILITY_UNSUPPORTED`
- CC-003 the negative control still bites: an id the profile does not list warns
"""

from __future__ import annotations

import json
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).parents[1] / "scripts"))
sys.path.insert(0, str(Path(__file__).parent))

from workspace_check import validate_capabilities  # noqa: E402

GOVERNANCE_ROOT = Path(__file__).resolve().parents[1]
PROFILES = GOVERNANCE_ROOT / "profiles"

CROSS_CUTTING = ("telemetry", "rate-limiting", "idempotency", "feature-flags")


def _profiles_with_supported_set() -> list[tuple[Path, dict]]:
    profiles = []
    for path in sorted(PROFILES.glob("*.json")):
        data = json.loads(path.read_text(encoding="utf-8"))
        if isinstance(data.get("supported_capabilities"), list):
            profiles.append((path, data))
    return profiles


def _supported(profile: str) -> set[str]:
    data = json.loads((PROFILES / f"{profile}.json").read_text(encoding="utf-8"))
    return set(data["supported_capabilities"])


class CrossCuttingCapabilityProfileTests(unittest.TestCase):
    def test_cc001_every_profile_with_a_supported_set_includes_the_cross_cutting_ids(self):
        profiles = _profiles_with_supported_set()
        self.assertTrue(profiles, "no profile declares a supported_capabilities list")
        for path, data in profiles:
            for capability in CROSS_CUTTING:
                self.assertIn(
                    capability,
                    data["supported_capabilities"],
                    f"{path.name} does not list the cross-cutting capability {capability!r}",
                )

    def test_cc002_a_cross_cutting_declaration_emits_no_unsupported_warning(self):
        findings: list = []
        validate_capabilities(
            {"telemetry": {"owner": "shared", "evidence_state": "declared"}},
            GOVERNANCE_ROOT,
            findings,
            "demo-project",
            supported=_supported("dotnet-product"),
        )
        self.assertEqual([item.code for item in findings], [])

    def test_cc003_a_capability_outside_the_supported_set_still_warns(self):
        findings: list = []
        validate_capabilities(
            {"billing": {"owner": "platform", "evidence_state": "declared"}},
            GOVERNANCE_ROOT,
            findings,
            "demo-project",
            supported=_supported("dotnet-library"),
        )
        self.assertIn("CAPABILITY_UNSUPPORTED", [item.code for item in findings])


if __name__ == "__main__":
    unittest.main()
