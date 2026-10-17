use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum JobStatus {
    Pending,
    Running,
    Success,
    Failed,
}

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JobStatus::Pending => write!(f, "Pending"),
            JobStatus::Running => write!(f, "Running"),
            JobStatus::Success => write!(f, "Success"),
            JobStatus::Failed => write!(f, "Failed"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixJob {
    pub id: usize,
    pub toolchain: Option<String>,
    pub features: Vec<String>,
    pub no_default_features: bool,
    pub status: JobStatus,
    pub log: String,
    pub duration_ms: u64,
    pub error_message: Option<String>,
}

impl MatrixJob {
    pub fn new(
        id: usize,
        toolchain: Option<String>,
        features: Vec<String>,
        no_default_features: bool,
    ) -> Self {
        Self {
            id,
            toolchain,
            features,
            no_default_features,
            status: JobStatus::Pending,
            log: String::new(),
            duration_ms: 0,
            error_message: None,
        }
    }

    pub fn display_features(&self) -> String {
        if self.features.is_empty() {
            if self.no_default_features {
                "none".to_string()
            } else {
                "default".to_string()
            }
        } else {
            let mut prefix = if self.no_default_features {
                String::new()
            } else {
                "default,".to_string()
            };
            prefix.push_str(&self.features.join(","));
            prefix
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatrixReport {
    pub project_name: String,
    pub timestamp: String,
    pub elapsed_ms: u64,
    pub total_jobs: usize,
    pub successful_jobs: usize,
    pub failed_jobs: usize,
    pub jobs: Vec<MatrixJob>,
}
