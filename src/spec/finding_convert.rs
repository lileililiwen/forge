//! # `DoctorFindingInput` - Trait Implementations
//!
//! This module contains trait implementations for `DoctorFindingInput`.
//!
//! ## Implemented Traits
//!
//! - `From`
//!
//! 🤖 Generated with [SplitRS](https://github.com/cool-japan/splitrs)

use crate::doctor::Finding;

use super::lifecycle::category_from_remediation;
use super::model::DoctorFindingInput;

impl From<&Finding> for DoctorFindingInput {
    fn from(f: &Finding) -> Self {
        DoctorFindingInput {
            id: f.id.clone(),
            status: f.status,
            remediation: f.remediation,
            category: category_from_remediation(f.remediation),
            detail: f.detail.clone(),
        }
    }
}
