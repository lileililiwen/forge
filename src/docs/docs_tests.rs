//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

#[cfg(test)]
pub(super) mod tests {
    use crate::core::manifest::Manifest;
    use crate::docs::assess::assess_freshness;
    use crate::docs::constants::{
        DOCS_CONTRACT_VERSION, STATUS_CURRENT, STATUS_FAILED, STATUS_TRANSLATED,
    };
    use crate::docs::model::{
        DocBlock, DocsConfig, FreshnessStatus, ReviewStatus, TranslateOutcome, TranslateReport,
        TranslateRequest, TranslationState, TranslatorConfig,
    };
    use crate::docs::source::{
        default_derivative_path, docs_config_from_manifest, extract_link_destinations, hash_bytes,
        load_translation_state, save_translation_state, segment_source, state_path_for,
        validate_locale, validate_output,
    };
    use crate::docs::translate::run_translate;
    use std::collections::BTreeMap;
    use std::ffi::OsString;
    use std::fs;
    use std::path::Path;
    use std::time::Duration;
    use tempfile::TempDir;

    fn manifest_with_docs(text: &str) -> Manifest {
        Manifest::parse(Path::new("forge.yaml"), text.as_bytes()).expect("manifest")
    }

    fn base_manifest(extra: &str) -> String {
        format!(
            "schema: 1\nproject:\n  id: docs-app\n  name: Docs\n  profile: rust-web\n  maturity: L1\nruntime:\n  language: rust\n{extra}"
        )
    }

    fn translator_config() -> TranslatorConfig {
        TranslatorConfig {
            binary: OsString::from("definitely-not-a-real-translator-xyz"),
            timeout: Duration::from_secs(5),
        }
    }

    fn write_project(dir: &Path, manifest_extra: &str, source: &str) {
        fs::create_dir_all(dir).unwrap();
        fs::write(dir.join("forge.yaml"), base_manifest(manifest_extra)).unwrap();
        fs::write(dir.join("README.md"), source).unwrap();
    }

    #[test]
    fn locale_validation_accepts_tags_and_refuses_paths() {
        for ok in ["en", "zh-CN", "pt-BR", "es-419"] {
            assert!(validate_locale(ok).is_ok(), "{ok}");
        }
        for bad in [
            "",
            "e",
            "toolonglanguage",
            "zh_CN",
            "zh CN",
            "../evil",
            "zh-CN/../../x",
            "zh.CN",
            "zh/CN",
            "-en",
            "en-",
            "en--US",
        ] {
            let err = validate_locale(bad).expect_err("must refuse");
            assert_eq!(err.code(), "docs-invalid", "{bad}");
        }
    }

    #[test]
    fn config_defaults_source_and_requires_opt_in() {
        let manifest = manifest_with_docs(&base_manifest(
            "docs:\n  source_language: en\n  translations:\n    zh-CN:\n      enabled: true\n    fr:\n      enabled: false\n",
        ));
        let config = DocsConfig::from_manifest_meta(&manifest.docs.unwrap()).unwrap();
        assert_eq!(config.source, "README.md");
        assert_eq!(config.source_language, "en");
        assert_eq!(config.locales.len(), 2);
        let enabled = config.enabled_locales();
        assert_eq!(enabled.len(), 1);
        assert_eq!(enabled[0].locale, "zh-CN");
    }

    #[test]
    fn config_honours_explicit_source_and_paths() {
        let manifest = manifest_with_docs(&base_manifest(
            "docs:\n  source: guide.md\n  non_translatable:\n    - Forge\n  translations:\n    zh-CN:\n      enabled: true\n      path: i18n/zh.md\n",
        ));
        let config = DocsConfig::from_manifest_meta(&manifest.docs.unwrap()).unwrap();
        assert_eq!(config.source, "guide.md");
        assert_eq!(config.non_translatable, vec!["Forge".to_string()]);
        assert_eq!(config.locales[0].path.as_deref(), Some("i18n/zh.md"));
    }

    #[test]
    fn config_refuses_empty_terms_and_bad_locales() {
        let manifest =
            manifest_with_docs(&base_manifest("docs:\n  non_translatable:\n    - '  '\n"));
        let err = DocsConfig::from_manifest_meta(&manifest.docs.unwrap()).expect_err("empty term");
        assert_eq!(err.code(), "docs-invalid");
        let manifest = manifest_with_docs(&base_manifest(
            "docs:\n  translations:\n    '../evil':\n      enabled: true\n",
        ));
        let err = DocsConfig::from_manifest_meta(&manifest.docs.unwrap()).expect_err("bad locale");
        assert_eq!(err.code(), "docs-invalid");
    }

    #[test]
    fn default_derivative_path_matches_brief_model() {
        assert_eq!(
            default_derivative_path("README.md", "zh-CN"),
            "docs/README.zh-CN.md"
        );
        assert_eq!(
            default_derivative_path("guide.md", "fr"),
            "docs/guide.fr.md"
        );
        assert_eq!(default_derivative_path("notes", "de"), "docs/notes.de.md");
    }

    #[test]
    fn segmentation_splits_paragraphs_and_preserves_code() {
        let text = "# Title\n\nFirst paragraph\nstill first.\n\n```sh\nforge list\n```\n\nSecond paragraph with [a link](https://example.com/x).\n";
        let blocks = segment_source(text);
        assert_eq!(blocks.len(), 4);
        assert!(matches!(&blocks[0], DocBlock::Text { id, .. } if id == "seg-000"));
        assert!(matches!(&blocks[1], DocBlock::Text { id, .. } if id == "seg-001"));
        assert!(matches!(&blocks[3], DocBlock::Text { id, .. } if id == "seg-002"));
        // Code block text is byte-identical to the source span.
        if let DocBlock::Code { text } = &blocks[2] {
            assert_eq!(text, "```sh\nforge list\n```");
        } else {
            panic!("expected code block at index 2");
        }
        let dests = extract_link_destinations(text);
        assert_eq!(dests, vec!["https://example.com/x".to_string()]);
    }

    #[test]
    fn output_validation_flags_dropped_links_and_terms() {
        let source = "See [the guide](https://example.com/guide) and run Forge doctor.\n";
        let good = "Voir [the guide](https://example.com/guide) and run Forge doctor.\n";
        assert!(validate_output(source, good, &["Forge".to_string()]).is_empty());
        let dropped_link = "Voir [the guide](https://example.com/other) and run Forge doctor.\n";
        let reasons = validate_output(source, dropped_link, &["Forge".to_string()]);
        assert!(
            reasons
                .iter()
                .any(|r| r.contains("https://example.com/guide")),
            "{reasons:?}"
        );
        let dropped_term = "Voir [the guide](https://example.com/guide) and run doctor.\n";
        let reasons = validate_output(source, dropped_term, &["Forge doctor".to_string()]);
        assert!(
            reasons.iter().any(|r| r.contains("Forge doctor")),
            "{reasons:?}"
        );
    }

    #[test]
    fn derivative_equal_to_source_is_refused_before_writing() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("proj");
        write_project(
            &dir,
            "docs:\n  translations:\n    zh-CN:\n      enabled: true\n      path: README.md\n",
            "Hello.\n",
        );
        let manifest = Manifest::load_from_dir(&dir, None).unwrap().0;
        let config = docs_config_from_manifest(&manifest).unwrap();
        let request = TranslateRequest {
            project_id: "docs-app".to_string(),
            locale: Some("zh-CN".to_string()),
            all: false,
        };
        let err = run_translate(&dir, &config, &request, &translator_config()).expect_err("refuse");
        assert_eq!(err.code(), "docs-invalid");
        assert!(err.to_string().contains("canonical source"));
        assert!(
            !dir.join(".forge/docs/zh-CN/state.json").exists(),
            "no state may be written on refusal"
        );
    }

    #[test]
    fn derivative_outside_project_is_refused_before_writing() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("proj");
        write_project(
            &dir,
            "docs:\n  translations:\n    zh-CN:\n      enabled: true\n      path: ../evil.md\n",
            "Hello.\n",
        );
        let manifest = Manifest::load_from_dir(&dir, None).unwrap().0;
        let config = docs_config_from_manifest(&manifest).unwrap();
        let request = TranslateRequest {
            project_id: "docs-app".to_string(),
            locale: Some("zh-CN".to_string()),
            all: false,
        };
        let err = run_translate(&dir, &config, &request, &translator_config()).expect_err("refuse");
        assert_eq!(err.code(), "docs-invalid");
        assert!(err.to_string().contains("outside the project"));
        assert!(!tmp.path().join("evil.md").exists());
    }

    #[test]
    fn unknown_and_disabled_locales_are_refused() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("proj");
        write_project(
            &dir,
            "docs:\n  translations:\n    zh-CN:\n      enabled: false\n",
            "Hello.\n",
        );
        let manifest = Manifest::load_from_dir(&dir, None).unwrap().0;
        let config = docs_config_from_manifest(&manifest).unwrap();
        for (locale, all, needle) in [
            (Some("fr".to_string()), false, "unknown locale"),
            (Some("zh-CN".to_string()), false, "is disabled"),
            (None, true, "no enabled translation locales"),
        ] {
            let request = TranslateRequest {
                project_id: "docs-app".to_string(),
                locale,
                all,
            };
            let err =
                run_translate(&dir, &config, &request, &translator_config()).expect_err("refuse");
            assert_eq!(err.code(), "docs-invalid", "{needle}");
            assert!(err.to_string().contains(needle), "{err}");
        }
    }

    #[test]
    fn missing_binary_fails_without_touching_prior_state() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("proj");
        write_project(
            &dir,
            "docs:\n  translations:\n    zh-CN:\n      enabled: true\n",
            "Hello world.\n",
        );
        let manifest = Manifest::load_from_dir(&dir, None).unwrap().0;
        let config = docs_config_from_manifest(&manifest).unwrap();
        let request = TranslateRequest {
            project_id: "docs-app".to_string(),
            locale: Some("zh-CN".to_string()),
            all: false,
        };
        // Seed a prior good state + derivative, then fail the run.
        let state_path = state_path_for(&dir, "zh-CN").unwrap();
        let prior = TranslationState {
            contract: DOCS_CONTRACT_VERSION.to_string(),
            locale: "zh-CN".to_string(),
            source: "README.md".to_string(),
            path: "docs/README.zh-CN.md".to_string(),
            source_hash: "old".to_string(),
            source_language: "en".to_string(),
            review: Some(ReviewStatus::Ok),
            review_reasons: Vec::new(),
            segments: BTreeMap::new(),
            translated_at: "2026-01-01T00:00:00Z".to_string(),
        };
        save_translation_state(&state_path, &prior).unwrap();
        fs::create_dir_all(dir.join("docs")).unwrap();
        fs::write(dir.join("docs/README.zh-CN.md"), "prior translation\n").unwrap();

        let report = run_translate(&dir, &config, &request, &translator_config()).unwrap();
        assert!(!report.healthy());
        assert_eq!(report.outcomes[0].status, STATUS_FAILED);
        assert!(report.outcomes[0].evidence.join(" ").contains("not found"));
        // Prior derivative and state are byte-identical.
        assert_eq!(
            fs::read_to_string(dir.join("docs/README.zh-CN.md")).unwrap(),
            "prior translation\n"
        );
        assert_eq!(load_translation_state(&state_path).unwrap(), prior);
    }

    #[test]
    fn state_round_trip_preserves_hashes_and_review() {
        let tmp = TempDir::new().unwrap();
        let path = state_path_for(tmp.path(), "zh-CN").unwrap();
        let mut segments = BTreeMap::new();
        segments.insert("abc".to_string(), "translated".to_string());
        let state = TranslationState {
            contract: DOCS_CONTRACT_VERSION.to_string(),
            source_hash: "deadbeef".to_string(),
            review: Some(ReviewStatus::NeedsReview),
            review_reasons: vec!["link destination `https://x` was altered".to_string()],
            segments,
            ..TranslationState::default()
        };
        save_translation_state(&path, &state).unwrap();
        assert_eq!(load_translation_state(&path).unwrap(), state);
    }

    #[test]
    fn request_validation_rejects_empty_and_ambiguous() {
        let empty = TranslateRequest {
            project_id: "".to_string(),
            locale: Some("zh-CN".to_string()),
            all: false,
        };
        assert_eq!(empty.validate().unwrap_err().code(), "docs-invalid");
        let missing = TranslateRequest {
            project_id: "x".to_string(),
            locale: None,
            all: false,
        };
        assert!(missing.validate().is_err());
        let both = TranslateRequest {
            project_id: "x".to_string(),
            locale: Some("zh-CN".to_string()),
            all: true,
        };
        assert!(both.validate().is_err());
        let bad_locale = TranslateRequest {
            project_id: "x".to_string(),
            locale: Some("../evil".to_string()),
            all: false,
        };
        assert_eq!(bad_locale.validate().unwrap_err().code(), "docs-invalid");
    }

    #[test]
    fn freshness_assessment_tracks_current_stale_and_never() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("proj");
        write_project(
            &dir,
            "docs:\n  translations:\n    zh-CN:\n      enabled: true\n    fr:\n      enabled: false\n",
            "Hello.\n",
        );
        let manifest = Manifest::load_from_dir(&dir, None).unwrap().0;
        // Never translated: derivative absent.
        let fresh = assess_freshness(&dir, &manifest).unwrap();
        assert_eq!(fresh.len(), 1, "disabled `fr` is skipped: {fresh:?}");
        assert_eq!(fresh[0].locale, "zh-CN");
        assert!(matches!(fresh[0].status, FreshnessStatus::NeverTranslated));
        // Stale: derivative exists but the hash moved.
        fs::create_dir_all(dir.join("docs")).unwrap();
        fs::write(dir.join("docs/README.zh-CN.md"), "old\n").unwrap();
        let state_path = state_path_for(&dir, "zh-CN").unwrap();
        save_translation_state(
            &state_path,
            &TranslationState {
                source_hash: "stale-hash".to_string(),
                ..TranslationState::default()
            },
        )
        .unwrap();
        let fresh = assess_freshness(&dir, &manifest).unwrap();
        assert!(matches!(fresh[0].status, FreshnessStatus::Stale));
        // Current: recorded hash matches.
        let current_hash = hash_bytes("Hello.\n".as_bytes());
        save_translation_state(
            &state_path,
            &TranslationState {
                source_hash: current_hash,
                review: Some(ReviewStatus::Ok),
                ..TranslationState::default()
            },
        )
        .unwrap();
        let fresh = assess_freshness(&dir, &manifest).unwrap();
        assert!(matches!(fresh[0].status, FreshnessStatus::Current { .. }));
        // Source edit flips back to stale.
        fs::write(dir.join("README.md"), "Hello changed.\n").unwrap();
        let fresh = assess_freshness(&dir, &manifest).unwrap();
        assert!(matches!(fresh[0].status, FreshnessStatus::Stale));
    }

    #[test]
    fn freshness_reports_misconfigured_source() {
        let tmp = TempDir::new().unwrap();
        let dir = tmp.path().join("proj");
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("forge.yaml"),
            base_manifest(
                "docs:\n  source: MISSING.md\n  translations:\n    zh-CN:\n      enabled: true\n",
            ),
        )
        .unwrap();
        // No source file on disk at all.
        let manifest = Manifest::load_from_dir(&dir, None).unwrap().0;
        let fresh = assess_freshness(&dir, &manifest).unwrap();
        assert_eq!(fresh.len(), 1);
        assert!(fresh[0].is_failure());
        assert_eq!(fresh[0].status_label(), "misconfigured");
    }

    #[test]
    fn report_health_requires_all_locales_ok() {
        let outcome = |status: &str| TranslateOutcome {
            locale: "zh-CN".to_string(),
            status: status.to_string(),
            source_hash: None,
            segments_translated: 0,
            segments_reused: 0,
            derivative: None,
            review: "ok".to_string(),
            review_reasons: Vec::new(),
            note: String::new(),
            evidence: Vec::new(),
            recovery: Vec::new(),
        };
        let healthy = TranslateReport {
            contract: DOCS_CONTRACT_VERSION.to_string(),
            project_id: "x".to_string(),
            source: "README.md".to_string(),
            source_language: "en".to_string(),
            locales: vec!["zh-CN".to_string()],
            all: false,
            state_dir: ".forge/docs".to_string(),
            outcomes: vec![outcome(STATUS_TRANSLATED), outcome(STATUS_CURRENT)],
            note: String::new(),
            healthy: true,
        };
        assert!(healthy.healthy());
        let partial = TranslateReport {
            outcomes: vec![outcome(STATUS_TRANSLATED), outcome(STATUS_FAILED)],
            healthy: false,
            ..healthy.clone()
        };
        assert!(!partial.healthy());
        let empty = TranslateReport {
            outcomes: Vec::new(),
            healthy: false,
            ..healthy.clone()
        };
        assert!(!empty.healthy());
    }
}
