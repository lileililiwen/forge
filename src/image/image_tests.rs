//! Unit tests for the delegated image build's validation rules.

#[cfg(test)]
pub(super) mod tests {
    use crate::image::*;
    use crate::image::{record_sbom, validate_response, write_record, BuilderResponse};
    use sha2::{Digest, Sha256};
    use std::path::{Path, PathBuf};

    const REV: &str = "0123456789abcdef0123456789abcdef01234567";

    #[test]
    fn the_default_tag_names_the_project_and_revision() {
        assert_eq!(default_tag("app", REV), "forge-app:0123456789ab");
    }

    #[test]
    fn an_image_tag_is_bounded_and_flag_safe() {
        assert!(validate_tag("forge-app:0123456789ab").is_ok());
        assert!(validate_tag("registry.example/app@sha256:abc").is_ok());
        for tag in ["", "-rm", ".hidden", "app name", "app;rm", &"a".repeat(256)] {
            assert!(validate_tag(tag).is_err(), "{tag:?} must be refused");
        }
    }

    #[test]
    fn only_a_sha256_digest_is_accepted() {
        let good = format!("sha256:{}", "a".repeat(64));
        assert!(validate_digest(&good).is_ok());
        for digest in [
            "".to_string(),
            "sha512:abc".to_string(),
            format!("sha256:{}", "A".repeat(64)),
            format!("sha256:{}", "a".repeat(63)),
        ] {
            assert!(
                validate_digest(&digest).is_err(),
                "{digest:?} must be refused"
            );
        }
    }

    #[test]
    fn a_project_relative_sbom_is_hashed_by_forge() {
        let dir = tempfile::tempdir().unwrap();
        let sbom_dir = dir.path().join(".forge/build/app");
        std::fs::create_dir_all(&sbom_dir).unwrap();
        let path = sbom_dir.join("sbom.cdx.json");
        std::fs::write(&path, b"{\"bomFormat\":\"CycloneDX\"}").unwrap();

        let record =
            record_sbom(dir.path(), ".forge/build/app/sbom.cdx.json").expect("a valid sbom");
        let expected = format!("{:x}", Sha256::digest(b"{\"bomFormat\":\"CycloneDX\"}"));
        assert_eq!(record.sha256, expected);
        assert_eq!(record.bytes, 25);
        assert_eq!(record.path, ".forge/build/app/sbom.cdx.json");
    }

    #[test]
    fn an_sbom_path_that_leaves_the_project_is_refused() {
        let dir = tempfile::tempdir().unwrap();
        let outside = dir.path().parent().unwrap().join("outside.json");
        std::fs::write(&outside, b"{}").unwrap();

        for raw in [
            outside.display().to_string(),
            "../outside.json".to_string(),
            "sbom/../../outside.json".to_string(),
            "missing.json".to_string(),
            "".to_string(),
        ] {
            assert!(
                record_sbom(dir.path(), &raw).is_err(),
                "{raw:?} must be refused"
            );
        }
    }

    #[test]
    fn an_oversized_sbom_is_refused_before_it_is_read() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("huge.json");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(SBOM_MAX_BYTES + 1).unwrap();
        drop(file);
        let err = record_sbom(dir.path(), "huge.json").unwrap_err();
        assert!(err.contains("over the"), "{err}");
    }

    #[test]
    fn evidence_is_bounded_and_redacted() {
        let mut lines: Vec<String> = (0..25).map(|n| format!("line {n}")).collect();
        lines.push(format!("token={}", "x".repeat(2000)));
        let bounded = bounded_evidence(&lines);
        assert!(bounded.len() <= EVIDENCE_MAX);
        for line in &bounded {
            assert!(line.chars().count() <= EVIDENCE_LINE_MAX);
        }
        let joined = bounded.join(" ");
        assert!(!joined.contains("token="), "{joined}");
    }

    #[test]
    fn a_response_is_checked_against_the_required_contract() {
        let dir = tempfile::tempdir().unwrap();
        let response = |contract: &str, image: &str, digest: &str| BuilderResponse {
            contract: contract.to_string(),
            builder: Some("pack".to_string()),
            builder_version: Some("0.35.0".to_string()),
            status: Some("built".to_string()),
            image: Some(image.to_string()),
            digest: Some(digest.to_string()),
            sbom_path: None,
            evidence: Vec::new(),
        };
        let tag = "forge-app:0123456789ab";
        let digest = format!("sha256:{}", "a".repeat(64));

        let newer = response("forge-image-builder/0.2.0", tag, &digest);
        assert!(validate_response(&newer, tag, dir.path()).is_ok());

        let next_major = response("forge-image-builder/1.0.0", tag, &digest);
        let err = validate_response(&next_major, tag, dir.path()).unwrap_err();
        assert!(err.contains("major versions differ"), "{err}");

        let other_family = response("forge-deploy-executor/0.1.0", tag, &digest);
        let err = validate_response(&other_family, tag, dir.path()).unwrap_err();
        assert!(err.contains("families differ"), "{err}");

        let wrong_image = response("forge-image-builder/0.1.0", "other:tag", &digest);
        let err = validate_response(&wrong_image, tag, dir.path()).unwrap_err();
        assert!(
            err.contains("while `forge-app:0123456789ab` was requested"),
            "{err}"
        );
    }

    #[test]
    fn the_record_path_is_project_relative_and_revision_scoped() {
        let path = record_path(Path::new("/workspace"), "app", REV);
        assert_eq!(
            path,
            PathBuf::from("/workspace/.forge/build/app/0123456789ab.json")
        );
    }

    #[test]
    fn the_adapter_resolution_prefers_the_explicit_flag() {
        std::env::remove_var(IMAGE_BUILDER_ENV);
        assert_eq!(resolve_adapter(Some("/opt/builder")), "/opt/builder");
        assert_eq!(resolve_adapter(Some("  ")), IMAGE_BUILDER_DEFAULT);
        assert_eq!(resolve_adapter(None), IMAGE_BUILDER_DEFAULT);
    }

    #[test]
    fn a_staged_record_replaces_the_previous_one_without_a_temporary_file() {
        let dir = tempfile::tempdir().unwrap();
        let digest = format!("sha256:{}", "a".repeat(64));
        let outcome = ImageBuildOutcome {
            contract: IMAGE_BUILD_RECORD_CONTRACT.to_string(),
            project_id: "app".to_string(),
            revision: REV.to_string(),
            tag: "forge-app:0123456789ab".to_string(),
            builder: "pack".to_string(),
            builder_version: None,
            digest,
            sbom: None,
            evidence: Vec::new(),
            observed_at: "2026-10-30T00:00:00Z".to_string(),
        };
        write_record(dir.path(), &outcome).expect("first record");
        let mut second = outcome.clone();
        second.digest = format!("sha256:{}", "b".repeat(64));
        write_record(dir.path(), &second).expect("second record");

        let path = record_path(dir.path(), "app", REV);
        let bytes = std::fs::read(&path).unwrap();
        let stored: ImageBuildOutcome = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(stored.digest, second.digest);
        let leftovers: Vec<_> = std::fs::read_dir(dir.path().join(".forge/build/app"))
            .unwrap()
            .filter_map(Result::ok)
            .filter(|entry| entry.file_name().to_string_lossy().contains(".tmp"))
            .collect();
        assert!(leftovers.is_empty(), "{leftovers:?}");
    }
}
