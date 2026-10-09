//! Auto-generated module
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use super::model::{GapStatus, RemediationClass};

/// Internal helper: extract the (subject, status, remediation, evidence,
/// detail) tuple from a rule.
pub(super) type RuleResult = (
    &'static str,
    GapStatus,
    RemediationClass,
    Vec<String>,
    String,
);
