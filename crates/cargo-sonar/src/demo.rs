use crate::model::{
    ClippyIssue, CodeLocation, CoverageMetric, IssueLevel, SecurityIssue,
    SecurityIssueCategory, SonarReport, UnsafeMetric,
};

pub fn generate_demo_report() -> SonarReport {
    let clippy_issues = vec![
        ClippyIssue {
            level: IssueLevel::Warning,
            lint_name: "clippy::redundant_clone".to_string(),
            message: "redundant clone".to_string(),
            location: CodeLocation {
                file: "src/auth.rs".to_string(),
                line: 42,
                column: 26,
                snippet: Some("let session_token = token.clone();".to_string()),
            },
        },
        ClippyIssue {
            level: IssueLevel::Warning,
            lint_name: "clippy::needless_borrow".to_string(),
            message: "this expression borrows a reference (&str) that is already a reference".to_string(),
            location: CodeLocation {
                file: "src/db/pool.rs".to_string(),
                line: 118,
                column: 41,
                snippet: Some("let conn = establish_connection(&&connection_str);".to_string()),
            },
        },
        ClippyIssue {
            level: IssueLevel::Error,
            lint_name: "compiler-error".to_string(),
            message: "mismatched types: expected struct `String`, found `&str`".to_string(),
            location: CodeLocation {
                file: "src/main.rs".to_string(),
                line: 25,
                column: 32,
                snippet: Some("let app_name: String = \"cargo-sonar\";".to_string()),
            },
        },
        ClippyIssue {
            level: IssueLevel::Note,
            lint_name: "clippy::todo".to_string(),
            message: "todo loop: implement token rotation strategy before production deployment".to_string(),
            location: CodeLocation {
                file: "src/auth.rs".to_string(),
                line: 154,
                column: 9,
                snippet: Some("todo!(\"implement token rotation\");".to_string()),
            },
        },
    ];

    let security_issues = vec![
        SecurityIssue {
            category: SecurityIssueCategory::Vulnerability,
            package_name: "openssl-src".to_string(),
            package_version: "1.11.22".to_string(),
            severity: "High".to_string(),
            message: "Use-after-free in SSL_do_handshake processing leading to remote code execution (CVE-2023-3446).".to_string(),
            advisory_url: Some("https://rustsec.org/advisories/RUSTSEC-2023-0052.html".to_string()),
        },
        SecurityIssue {
            category: SecurityIssueCategory::LicenseViolation,
            package_name: "gpl-helper".to_string(),
            package_version: "0.4.1".to_string(),
            severity: "Medium".to_string(),
            message: "Crate is licensed under GPL-3.0, which violates the allowed license policy (MIT, Apache-2.0).".to_string(),
            advisory_url: None,
        },
        SecurityIssue {
            category: SecurityIssueCategory::BannedCrate,
            package_name: "lazy_static".to_string(),
            package_version: "1.4.0".to_string(),
            severity: "Low".to_string(),
            message: "lazy_static is banned; please transition to standard library's std::sync::OnceLock.".to_string(),
            advisory_url: None,
        },
    ];

    let coverage = vec![
        CoverageMetric {
            file: "src/main.rs".to_string(),
            covered_lines: 48,
            total_lines: 64,
            percentage: 75.0,
        },
        CoverageMetric {
            file: "src/auth.rs".to_string(),
            covered_lines: 112,
            total_lines: 215,
            percentage: 52.09,
        },
        CoverageMetric {
            file: "src/db/pool.rs".to_string(),
            covered_lines: 84,
            total_lines: 88,
            percentage: 95.45,
        },
        CoverageMetric {
            file: "src/utils.rs".to_string(),
            covered_lines: 45,
            total_lines: 45,
            percentage: 100.0,
        },
    ];

    let unsafe_metrics = vec![
        UnsafeMetric {
            file: "src/main.rs".to_string(),
            unsafe_functions: 0,
            unsafe_impls: 0,
            unsafe_blocks: 0,
        },
        UnsafeMetric {
            file: "src/auth.rs".to_string(),
            unsafe_functions: 0,
            unsafe_impls: 0,
            unsafe_blocks: 0,
        },
        UnsafeMetric {
            file: "src/db/pool.rs".to_string(),
            unsafe_functions: 1,
            unsafe_impls: 0,
            unsafe_blocks: 2,
        },
    ];

    // Compute Health grade using our standard calculation engine
    SonarReport::calculate_grade(
        "Cargo Sonar (Simulated)".to_string(),
        &clippy_issues,
        &security_issues,
        &coverage,
        &unsafe_metrics,
        14, // Total workspace files mock
    )
}
