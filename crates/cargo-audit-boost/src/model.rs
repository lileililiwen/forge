use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildStatus {
    Pending,
    Compiling,
    Completed,
    Failed,
}

#[derive(Debug, Clone)]
pub struct CrateBuildInfo {
    pub name: String,
    pub version: String,
    pub start_time: Option<Instant>,
    pub end_time: Option<Instant>,
    pub status: BuildStatus,
    #[allow(dead_code)]
    pub diagnostics: Vec<String>,
    pub cached: bool,
}

impl CrateBuildInfo {
    pub fn new(name: String, version: String) -> Self {
        Self {
            name,
            version,
            start_time: None,
            end_time: None,
            status: BuildStatus::Pending,
            diagnostics: Vec::new(),
            cached: false,
        }
    }

    pub fn duration(&self, now: Instant) -> Duration {
        match (self.start_time, self.end_time) {
            (Some(start), Some(end)) => end.duration_since(start),
            (Some(start), None) => now.duration_since(start),
            _ => Duration::from_secs(0),
        }
    }
}

pub struct BuildState {
    pub start_time: Instant,
    pub crates: HashMap<String, CrateBuildInfo>,
    pub crate_order: Vec<String>, // Preserves the order of compile discovery
    pub success: Option<bool>,
    pub max_concurrency: usize,
}

impl BuildState {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            crates: HashMap::new(),
            crate_order: Vec::new(),
            success: None,
            max_concurrency: 0,
        }
    }

    pub fn get_or_create(&mut self, name: &str, version: &str) -> &mut CrateBuildInfo {
        if !self.crates.contains_key(name) {
            self.crates.insert(
                name.to_string(),
                CrateBuildInfo::new(name.to_string(), version.to_string()),
            );
            self.crate_order.push(name.to_string());
        }
        self.crates.get_mut(name).unwrap()
    }

    pub fn add_diagnostic(&mut self, name: &str, version: &str, diagnostic: String) {
        let info = self.get_or_create(name, version);
        info.diagnostics.push(diagnostic);
    }

    pub fn update_concurrency(&mut self) {
        let current = self
            .crates
            .values()
            .filter(|c| c.status == BuildStatus::Compiling)
            .count();
        if current > self.max_concurrency {
            self.max_concurrency = current;
        }
    }

    pub fn mark_compiling(&mut self, name: &str, version: &str, now: Instant) {
        let info = self.get_or_create(name, version);
        if info.status == BuildStatus::Pending {
            info.status = BuildStatus::Compiling;
            info.start_time = Some(now);
        }
        self.update_concurrency();
    }

    pub fn mark_completed(&mut self, name: &str, version: &str, now: Instant, cached: bool) {
        let info = self.get_or_create(name, version);
        info.status = BuildStatus::Completed;
        if info.start_time.is_none() {
            info.start_time = Some(now); // Fallback if start wasn't captured
        }
        info.end_time = Some(now);
        info.cached = cached;
        self.update_concurrency();
    }

    pub fn mark_failed(&mut self, name: &str, version: &str, now: Instant) {
        let info = self.get_or_create(name, version);
        info.status = BuildStatus::Failed;
        if info.start_time.is_none() {
            info.start_time = Some(now);
        }
        info.end_time = Some(now);
        self.update_concurrency();
    }

    pub fn get_slowest_crates(&self, top_n: usize) -> Vec<(String, Duration)> {
        let now = Instant::now();
        let mut list: Vec<(String, Duration)> = self
            .crates
            .values()
            .filter(|c| c.status != BuildStatus::Pending)
            .map(|c| (c.name.clone(), c.duration(now)))
            .collect();

        list.sort_by(|a, b| b.1.cmp(&a.1));
        list.truncate(top_n);
        list
    }
}
