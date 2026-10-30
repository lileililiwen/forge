use crate::model::BuildState;
use anyhow::Result;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

pub async fn run_demo_simulation(state: Arc<Mutex<BuildState>>) -> Result<()> {
    // Phase 1: Start low-level foundational crates
    {
        let mut g = state.lock().await;
        g.mark_compiling("proc-macro2", "1.0.106", Instant::now());
        g.mark_compiling("unicode-ident", "1.0.24", Instant::now());
        g.mark_compiling("libc", "0.2.186", Instant::now());
    }
    sleep(Duration::from_millis(600)).await;

    // unicode-ident finishes, quote starts
    {
        let mut g = state.lock().await;
        g.mark_completed("unicode-ident", "1.0.24", Instant::now(), true); // Cached!
        g.mark_compiling("quote", "1.0.45", Instant::now());
    }
    sleep(Duration::from_millis(400)).await;

    // proc-macro2 finishes, syn starts
    {
        let mut g = state.lock().await;
        g.mark_completed("proc-macro2", "1.0.106", Instant::now(), true); // Cached!
        g.mark_compiling("syn", "2.0.118", Instant::now());
    }
    sleep(Duration::from_millis(700)).await;

    // libc finishes, serde and tokio start
    {
        let mut g = state.lock().await;
        g.mark_completed("libc", "0.2.186", Instant::now(), false);
        g.mark_compiling("serde", "1.0.228", Instant::now());
        g.mark_compiling("tokio", "1.52.3", Instant::now());
    }
    sleep(Duration::from_millis(500)).await;

    // quote finishes
    {
        let mut g = state.lock().await;
        g.mark_completed("quote", "1.0.45", Instant::now(), false);
    }
    sleep(Duration::from_millis(900)).await;

    // serde finishes, serde_json and futures start
    {
        let mut g = state.lock().await;
        g.mark_completed("serde", "1.0.228", Instant::now(), false);
        g.mark_compiling("serde_json", "1.0.150", Instant::now());
        g.mark_compiling("futures", "0.3.32", Instant::now());
    }
    sleep(Duration::from_millis(600)).await;

    // syn finishes, ratatui starts
    {
        let mut g = state.lock().await;
        g.mark_completed("syn", "2.0.118", Instant::now(), false);
        g.mark_compiling("ratatui", "0.26.3", Instant::now());
    }
    sleep(Duration::from_millis(800)).await;

    // tokio and serde_json finish
    {
        let mut g = state.lock().await;
        g.mark_completed("tokio", "1.52.3", Instant::now(), false);
        g.mark_completed("serde_json", "1.0.150", Instant::now(), false);
    }
    sleep(Duration::from_millis(400)).await;

    // futures finishes
    {
        let mut g = state.lock().await;
        g.mark_completed("futures", "0.3.32", Instant::now(), false);
    }
    sleep(Duration::from_millis(700)).await;

    // ratatui finishes, cargo-audit-boost starts
    {
        let mut g = state.lock().await;
        g.mark_completed("ratatui", "0.26.3", Instant::now(), false);
        g.mark_compiling("cargo-audit-boost", "0.1.0", Instant::now());
    }
    sleep(Duration::from_millis(900)).await;

    // cargo-audit-boost finishes
    {
        let mut g = state.lock().await;
        g.mark_completed("cargo-audit-boost", "0.1.0", Instant::now(), false);
        g.success = Some(true);
    }

    Ok(())
}
