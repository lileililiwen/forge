#!/usr/bin/env python3
"""Validation and normalization for repository discoverability declarations."""

from __future__ import annotations

import re
from dataclasses import dataclass
from typing import Any, Callable
from urllib.parse import urlparse


REQUIRED_DISCOVERABILITY_FIELDS = (
    "description",
    "keywords",
    "topics",
    "audience",
    "use_cases",
    "search_phrases",
    "homepage",
    "license_spdx",
    "visibility",
)
VISIBILITIES = {"public", "private", "internal"}
TOPIC = re.compile(r"^[a-z0-9][a-z0-9-]{0,49}$")
SPDX = re.compile(r"^[A-Za-z0-9][A-Za-z0-9.+-]*$")


@dataclass(frozen=True)
class MetadataFinding:
    level: str
    code: str
    message: str
    project: str | None = None


def _emit(findings: list[Any], level: str, code: str, message: str, project_id: str) -> None:
    """Append the repository checker Finding type without importing it eagerly."""
    try:
        from workspace_check import Finding
    except ImportError:
        findings.append(MetadataFinding(level, code, message, project_id))
    else:
        findings.append(Finding(level, code, message, project_id))


def normalize_topic(value: str) -> str:
    return re.sub(r"[^a-z0-9]+", "-", value.strip().lower()).strip("-")


def normalize_topics(values: list[str]) -> list[str]:
    result: list[str] = []
    for value in values:
        normalized = normalize_topic(value)
        if normalized and normalized not in result:
            result.append(normalized)
    return result


def _require_strings(value: Any, field: str) -> list[str]:
    if not isinstance(value, list) or not value or any(not isinstance(item, str) or not item.strip() for item in value):
        raise ValueError(f"{field} must be a non-empty array of non-empty strings")
    return [item.strip() for item in value]


def build_discoverability(
    *,
    description: str,
    keywords: list[str],
    topics: list[str],
    audience: list[str],
    use_cases: list[str],
    search_phrases: list[str],
    homepage: str,
    license_spdx: str,
    visibility: str,
) -> dict[str, Any]:
    if not isinstance(description, str) or not description.strip():
        raise ValueError("description must be a non-empty string")
    clean_keywords = _require_strings(keywords, "keywords")
    clean_topics = normalize_topics(_require_strings(topics, "topics"))
    if not clean_topics or any(not TOPIC.fullmatch(topic) for topic in clean_topics):
        raise ValueError("topics must normalize to lowercase GitHub topic names")
    clean_audience = _require_strings(audience, "audience")
    clean_use_cases = _require_strings(use_cases, "use_cases")
    clean_search_phrases = _require_strings(search_phrases, "search_phrases")
    if not isinstance(homepage, str) or not homepage.strip():
        raise ValueError("homepage must be a non-empty URL")
    parsed = urlparse(homepage.strip())
    if parsed.scheme not in {"http", "https"} or not parsed.netloc:
        raise ValueError("homepage must be an absolute http(s) URL")
    if not isinstance(license_spdx, str) or not SPDX.fullmatch(license_spdx.strip()):
        raise ValueError("license_spdx must be a valid SPDX identifier")
    if visibility not in VISIBILITIES:
        raise ValueError(f"visibility must be one of {sorted(VISIBILITIES)}")
    return {
        "description": description.strip(),
        "keywords": clean_keywords,
        "topics": clean_topics,
        "audience": clean_audience,
        "use_cases": clean_use_cases,
        "search_phrases": clean_search_phrases,
        "homepage": homepage.strip(),
        "license_spdx": license_spdx.strip(),
        "visibility": visibility,
    }


def validate_discoverability(declaration: dict[str, Any], project_path: Any, findings: list[Any], project_id: str) -> None:
    del project_path
    value = declaration.get("discoverability")
    if value is None:
        _emit(findings, "ERROR", "DISCOVERABILITY_MISSING", "project must declare discoverability metadata", project_id)
        return
    if not isinstance(value, dict):
        _emit(findings, "ERROR", "DISCOVERABILITY_INVALID", "discoverability must be an object", project_id)
        return
    missing = [field for field in REQUIRED_DISCOVERABILITY_FIELDS if field not in value]
    if missing:
        _emit(findings, "ERROR", "DISCOVERABILITY_INCOMPLETE", f"missing discoverability fields: {', '.join(missing)}", project_id)
        return
    try:
        built = build_discoverability(**{field: value[field] for field in REQUIRED_DISCOVERABILITY_FIELDS})
    except (TypeError, ValueError) as exc:
        _emit(findings, "ERROR", "DISCOVERABILITY_INVALID", str(exc), project_id)
        return
    if built != value:
        _emit(findings, "ERROR", "DISCOVERABILITY_INVALID", "discoverability values must be normalized and trimmed", project_id)


def validate_publication(declaration: dict[str, Any], findings: list[Any], project_id: str) -> None:
    publication = declaration.get("publication")
    if not isinstance(publication, dict) or publication.get("state") != "metadata_verified":
        _emit(findings, "ERROR", "PUBLICATION_UNVERIFIED", "GitHub publication evidence is not metadata_verified", project_id)
