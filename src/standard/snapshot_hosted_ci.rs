//! Hosted CI standard pack descriptors kept out of the core catalog module.

use super::layout::*;
use super::model::{PackDescriptor, PackEvidence, PackSupportState};
use super::snapshot::{build_pack, profile_declaration};

pub(super) fn client_1_2() -> PackDescriptor {
    build_pack(
        CLIENT_PACK,
        "1.2.0",
        PackSupportState::Supported,
        PackEvidence::Verified,
        CLIENT_PROFILES,
        None,
        &[
            (PROFILE_PATH, profile_declaration()),
            (
                VERIFY_PATH,
                include_str!("../../standards/baseline-client/1.2.0/verify.sh"),
            ),
            (CI_PATH, HOSTED_CI_CLIENT_WORKFLOW),
            (GITHUB_CI_PATH, HOSTED_CI_CLIENT_WORKFLOW),
            (INSTALL_FORGE_PATH, HOSTED_CI_CLIENT_INSTALLER),
            (
                PRE_COMMIT_PATH,
                include_str!("../../standards/baseline-client/1.2.0/.pre-commit-config.yaml"),
            ),
            (
                ".standard/scripts/upload-hosted-ci-evidence.py",
                HOSTED_CI_UPLOADER,
            ),
            (QUALITY_PATH, QUALITY_TEMPLATE),
        ],
    )
}

pub(super) fn client_1_3() -> PackDescriptor {
    build_pack(
        CLIENT_PACK,
        "1.3.0",
        PackSupportState::Supported,
        PackEvidence::Verified,
        CLIENT_PROFILES,
        None,
        &[
            (PROFILE_PATH, profile_declaration()),
            (
                VERIFY_PATH,
                include_str!("../../standards/baseline-client/1.3.0/verify.sh"),
            ),
            (
                CI_PATH,
                include_str!("../../standards/baseline-client/1.3.0/ci/verify.yml"),
            ),
            (
                GITHUB_CI_PATH,
                include_str!("../../standards/baseline-client/1.3.0/ci/verify.yml"),
            ),
            (
                INSTALL_FORGE_PATH,
                include_str!("../../standards/baseline-client/1.3.0/install-forge.sh"),
            ),
            (
                PRE_COMMIT_PATH,
                include_str!("../../standards/baseline-client/1.3.0/.pre-commit-config.yaml"),
            ),
            (
                ".standard/scripts/upload-hosted-ci-evidence.py",
                include_str!("../../standards/baseline-client/1.3.0/upload-hosted-ci-evidence.py"),
            ),
            (QUALITY_PATH, QUALITY_TEMPLATE),
        ],
    )
}

pub(super) fn service_1_5() -> PackDescriptor {
    build_pack(
        BASELINE_PACK,
        "1.5.0",
        PackSupportState::Supported,
        PackEvidence::Verified,
        BASELINE_PROFILES,
        None,
        &[
            (PROFILE_PATH, profile_declaration()),
            (VERIFY_PATH, VERIFY_WRAPPER_TEMPLATE),
            (VERIFY_LOCAL_PATH, SHARED_VERIFY_SCRIPT),
            (CI_PATH, HOSTED_CI_SERVICE_WORKFLOW),
            (GITHUB_CI_PATH, HOSTED_CI_SERVICE_WORKFLOW),
            (INSTALL_FORGE_PATH, HOSTED_CI_SERVICE_INSTALLER),
            (PRE_COMMIT_PATH, PRE_COMMIT_TEMPLATE),
            (
                ".standard/scripts/upload-hosted-ci-evidence.py",
                HOSTED_CI_UPLOADER,
            ),
            (QUALITY_PATH, QUALITY_TEMPLATE),
            (COMPOSE_PATH, COMPOSE_TEMPLATE),
            (OPENSPEC_NAME_CHECK_PATH, SHARED_OPENSPEC_NAME_CHECK),
            (ACCESSIBILITY_CHECK_PATH, SHARED_ACCESSIBILITY_CHECK),
            (PRIVACY_CHECK_PATH, SHARED_PRIVACY_CHECK),
        ],
    )
}

pub(super) fn service_1_6() -> PackDescriptor {
    build_pack(
        BASELINE_PACK,
        "1.6.0",
        PackSupportState::Supported,
        PackEvidence::Verified,
        BASELINE_PROFILES,
        None,
        &[
            (PROFILE_PATH, profile_declaration()),
            (VERIFY_PATH, VERIFY_WRAPPER_TEMPLATE),
            (VERIFY_LOCAL_PATH, SHARED_VERIFY_SCRIPT),
            (
                CI_PATH,
                include_str!("../../standards/baseline-service/1.6.0/ci/verify.yml"),
            ),
            (
                GITHUB_CI_PATH,
                include_str!("../../standards/baseline-service/1.6.0/ci/verify.yml"),
            ),
            (
                INSTALL_FORGE_PATH,
                include_str!("../../standards/baseline-service/1.6.0/install-forge.sh"),
            ),
            (
                PRE_COMMIT_PATH,
                include_str!("../../standards/baseline-service/1.6.0/.pre-commit-config.yaml"),
            ),
            (
                ".standard/scripts/upload-hosted-ci-evidence.py",
                include_str!("../../standards/baseline-service/1.6.0/upload-hosted-ci-evidence.py"),
            ),
            (QUALITY_PATH, QUALITY_TEMPLATE),
            (COMPOSE_PATH, COMPOSE_TEMPLATE),
            (OPENSPEC_NAME_CHECK_PATH, SHARED_OPENSPEC_NAME_CHECK),
            (ACCESSIBILITY_CHECK_PATH, SHARED_ACCESSIBILITY_CHECK),
            (PRIVACY_CHECK_PATH, SHARED_PRIVACY_CHECK),
        ],
    )
}
