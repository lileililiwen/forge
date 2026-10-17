# Cargo Sonar Boost 🛡️

A zero-config, high-performance code quality and technical debt dashboard CLI tool for Rust developers. It aggregates output from standard tools (`clippy`, `cargo-deny`, `cargo-geiger`, and `tarpaulin`) and compiles them into a premium, interactive, responsive local HTML dashboard.

---

## 🌟 Key Features

*   **Unified Health Rating**: Computes an overall codebase score (0-100) and grade (A to F) based on lints, security, test coverage, and memory safety metrics.
*   **Clippy Diagnostics Browser**: Review compiler diagnostics in a clean, visual format, complete with code snippets and search/severity filtering.
*   **Security & Dependency Auditing**: Checks for high-severity package vulnerabilities, banned crates, and license compliance mismatches.
*   **Line-Level Test Coverage Visualizer**: Displays a breakdown of test coverage percentages across all workspace source files.
*   **Unsafe Code Tracker**: Highlights files containing unsafe blocks, unsafe functions, and unsafe traits.
*   **Light/Dark Premium UI**: A highly responsive HSL-tailored browser dashboard with localStorage state saving, SVG radial gauges, and smooth transition animations.
*   **Simulation Mode (`--demo`)**: Instantly evaluate the dashboard visually using prepackaged mock data.

---

## 🛠️ Quick Start & Installation

### Prerequisites

Make sure you have standard Rust toolchains installed:
```bash
rustc --version
```

### Installation

1.  Clone this repository:
    ```bash
    git clone https://github.com/lileililiwen/cargo-sonar-boost.git
    cd cargo-sonar-boost
    ```
2.  Install the binary locally:
    ```bash
    cargo install --path .
    ```

---

## 🚀 Usage Guide

### 1. Try a Simulated Demo
Compile the project and inspect the visual HTML dashboard layout using dummy metrics immediately:
```bash
cargo sonar-boost --demo
```
This writes the reports to `target/sonar/` and opens your default system browser automatically.

### 2. Run a Real Codebase Quality Audit
Execute `cargo-sonar-boost` in the root of any Rust project:
```bash
# Run all quality checks and open the dashboard
cargo sonar-boost

# Run headlessly (e.g. in CI or SSH sessions) and write to a custom directory
cargo sonar-boost --no-open --output-dir target/reports
```

> **Note**: `cargo-sonar-boost` calls underlying check tools. If any tools are not installed locally (e.g., `cargo-deny`), the audit will skip them gracefully and flag warnings in your terminal logs.

---

## 📂 Exported Output Files

Upon completion of a check, the following files are exported:
*   **`target/sonar/report.json`**: Standalone JSON database of code metrics, issues, and coverage.
*   **`target/sonar/report.html`**: The premium, self-contained HTML visual dashboard.

---

## 📦 Optional Tool Enhancements

To populate all dashboard dimensions (Security, Safety, Coverage), install the corresponding cargo tools:

```bash
# For dependency and license audits
cargo install cargo-deny

# For unsafe code tracking
cargo install cargo-geiger

# For unit test coverage calculation
cargo install cargo-tarpaulin
```

---

## 📄 License

Licensed under the MIT License. See [LICENSE](LICENSE) for details.
