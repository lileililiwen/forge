# Cargo Matrix Boost 🚀

A cargo subcommand plugin (`cargo matrix-boost`) that checks, compiles, and runs tests across a matrix of toolchains (stable, nightly, beta) and feature flag combinations (powersets or custom sets). It orchestrates job execution concurrently and provides a real-time terminal UI (TUI) and an interactive local HTML dashboard.

---

## 🌟 Key Features

*   **Feature Powersets**: Automatically extracts workspace features using `cargo_metadata` and computes feature combinations.
*   **Multi-Toolchain Support**: Easily run checks across different compiler versions (`stable`, `nightly`, `beta`) using `rustup`.
*   **Throttled Asynchronous Execution**: Parallel execution throttled by default based on available CPU cores.
*   **Interactive TUI Grid**: Real-time progress grid mapping features against toolchains.
*   **Premium HTML Reports**: Standalone interactive HTML report with light/dark theme toggle, diagnostic logging, and test matrix grid.
*   **Simulation Mode (`--demo`)**: Instantly view the TUI grid and HTML dashboard using prepackaged simulation data.

---

## 🛠️ Quick Start & Installation

### Installation

1.  Clone this repository:
    ```bash
    git clone https://github.com/lileililiwen/forge.git
    cd forge
    ```
2.  Install the binary locally:
    ```bash
    cargo install --path crates/cargo-matrix-boost
    ```

---

## 🚀 Usage Guide

### 1. Try a Simulated Demo
Inspect the visual TUI and generated HTML report using simulated test matrix runs:
```bash
cargo matrix-boost --demo
```

### 2. Run a Real Codebase Matrix Verification
Execute `cargo-matrix-boost` in the root of any Rust project:
```bash
# Run feature combinations validation
cargo matrix-boost

# Run specific toolchains
cargo matrix-boost --toolchains stable,nightly

# Exclude default features or specify feature flags manually
cargo matrix-boost --features serde,tokio --exclude-features default

# Run headlessly (e.g. in CI/CD) and write output to custom directory
cargo matrix-boost --no-open --output-dir target/reports
```

---

## 📂 Exported Output Files

*   **`target/matrix/report.json`**: Standalone JSON database of feature/toolchain configurations and execution results.
*   **`target/matrix/report.html`**: The premium, self-contained HTML visual dashboard.

---

## 📄 License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.
