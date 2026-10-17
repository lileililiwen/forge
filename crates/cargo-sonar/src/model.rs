use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum IssueLevel {
    Error,
    Warning,
    Help,
    Note,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeLocation {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub snippet: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClippyIssue {
    pub level: IssueLevel,
    pub lint_name: String,
    pub message: String,
    pub location: CodeLocation,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SecurityIssueCategory {
    Vulnerability,
    LicenseViolation,
    BannedCrate,
    Unclassified,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SecurityIssue {
    pub category: SecurityIssueCategory,
    pub package_name: String,
    pub package_version: String,
    pub severity: String,
    pub message: String,
    pub advisory_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CoverageMetric {
    pub file: String,
    pub covered_lines: usize,
    pub total_lines: usize,
    pub percentage: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UnsafeMetric {
    pub file: String,
    pub unsafe_functions: usize,
    pub unsafe_impls: usize,
    pub unsafe_blocks: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodeHealthSummary {
    pub project_name: String,
    pub timestamp: DateTime<Utc>,
    pub overall_score: f64, // 0.0 - 100.0
    pub health_grade: String, // A, B, C, D, F
    
    // Totals
    pub total_files: usize,
    pub total_clippy_errors: usize,
    pub total_clippy_warnings: usize,
    pub total_security_vulnerabilities: usize,
    pub total_license_issues: usize,
    pub total_unsafe_blocks: usize,
    pub overall_coverage: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SonarReport {
    pub summary: CodeHealthSummary,
    pub clippy_issues: Vec<ClippyIssue>,
    pub security_issues: Vec<SecurityIssue>,
    pub coverage: Vec<CoverageMetric>,
    pub unsafe_metrics: Vec<UnsafeMetric>,
}

impl SonarReport {
    /// Computes the final health score and letter grade based on the raw metrics.
    pub fn calculate_grade(
        project_name: String,
        clippy_issues: &[ClippyIssue],
        security_issues: &[SecurityIssue],
        coverage: &[CoverageMetric],
        unsafe_metrics: &[UnsafeMetric],
        total_files: usize,
    ) -> SonarReport {
        let timestamp = Utc::now();

        // 1. Clippy Score (20% weight)
        // Starts at 100, drops for each warning/error
        let total_clippy_errors = clippy_issues.iter().filter(|i| matches!(i.level, IssueLevel::Error)).count();
        let total_clippy_warnings = clippy_issues.iter().filter(|i| matches!(i.level, IssueLevel::Warning)).count();
        let clippy_deduction = (total_clippy_errors as f64 * 5.0) + (total_clippy_warnings as f64 * 1.5);
        let clippy_score = (100.0 - clippy_deduction).max(0.0);

        // 2. Security Score (25% weight)
        // Starts at 100, drops severely for vulnerability/license issues
        let total_security_vulnerabilities = security_issues.iter().filter(|i| matches!(i.category, SecurityIssueCategory::Vulnerability)).count();
        let total_license_issues = security_issues.iter().filter(|i| matches!(i.category, SecurityIssueCategory::LicenseViolation)).count();
        let security_deduction = (total_security_vulnerabilities as f64 * 30.0) + (total_license_issues as f64 * 15.0);
        let security_score = (100.0 - security_deduction).max(0.0);

        // 3. Coverage Score (35% weight)
        // Simply the average coverage percentage across all files (or overall)
        let (total_covered, total_lines) = coverage.iter().fold((0, 0), |acc, c| (acc.0 + c.covered_lines, acc.1 + c.total_lines));
        let overall_coverage = if total_lines > 0 {
            (total_covered as f64 / total_lines as f64) * 100.0
        } else {
            100.0 // Default to 100% if no lines to cover (or tests have 100% success on files they run on)
        };
        let coverage_score = overall_coverage;

        // 4. Unsafe Score (20% weight)
        // Start at 100, deduct based on count of unsafe blocks per file
        let total_unsafe_blocks = unsafe_metrics.iter().map(|u| u.unsafe_blocks).sum::<usize>();
        let unsafe_deduction = (total_unsafe_blocks as f64 * 2.0).min(100.0);
        let unsafe_score = 100.0 - unsafe_deduction;

        // Overall Weighted Score
        let overall_score = (clippy_score * 0.20) + (security_score * 0.25) + (coverage_score * 0.35) + (unsafe_score * 0.20);

        // Grade mapping
        let health_grade = match overall_score {
            s if s >= 90.0 => "A".to_string(),
            s if s >= 80.0 => "B".to_string(),
            s if s >= 70.0 => "C".to_string(),
            s if s >= 60.0 => "D".to_string(),
            _ => "F".to_string(),
        };

        let summary = CodeHealthSummary {
            project_name,
            timestamp,
            overall_score,
            health_grade,
            total_files,
            total_clippy_errors,
            total_clippy_warnings,
            total_security_vulnerabilities,
            total_license_issues,
            total_unsafe_blocks,
            overall_coverage,
        };

        SonarReport {
            summary,
            clippy_issues: clippy_issues.to_vec(),
            security_issues: security_issues.to_vec(),
            coverage: coverage.to_vec(),
            unsafe_metrics: unsafe_metrics.to_vec(),
        }
    }
}
