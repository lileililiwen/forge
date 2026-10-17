# Cargo Audit Boost 🚀

A blazing fast cargo build performance profiling, timing, and compilation metrics visualizer. It provides an interactive Terminal UI (TUI) dashboard to monitor builds in real-time, and exports a premium interactive HTML report containing a detailed Gantt timeline, build metrics, compiler diagnostics, and a concurrency threads chart.

---

## 🌟 Key Features

### 🖥️ Interactive Terminal UI (TUI)
- **Live Gantt Timeline**: Monitor compilation progress crate-by-crate.
- **Split Dashboard View**: Sidebar showing selected crate details (exact duration, cached status, performance bottlenecks, and scrollable warning/error logs).
- **Real-Time Search**: Press `/` or `s` to filter compilation list live.
- **Timeline Toggles**: Hide/show cached crates (`c`) or focus strictly on compilation bottlenecks (`b`).

### 📊 Premium Interactive HTML Report
- **SVG Concurrency Graph**: View threads concurrency count timeline plotted with a beautiful gradient area chart.
- **Client-Side Live Filter & Sort**: Dynamically filter crates (All, Compiled, Cached, Failed, Bottlenecks) or sort (by Start Time, Duration, Name) in your browser.
- **Light/Dark Mode**: Clean color palettes with theme preferences saved in `localStorage`.
- **Diagnostics Browser**: Inspect warnings and error logs directly inside expandable logs sections formatted as terminal outputs.

---

## 🛠️ Quick Start & Installation

### Prerequisites
Make sure you have Rust and Cargo installed:
```bash
rustc --version
```

### Installation
1. Clone this repository:
   ```bash
   git clone https://github.com/username/cargo-audit-boost.git
   cd cargo-audit-boost
   ```
2. Build and install the binary globally:
   ```bash
   cargo install --path .
   ```

---

## 🚀 Usage Guide

### 1. Run a Simulated Demo
Run a simulated compilation profiling build scan to see all features in action:
```bash
cargo-audit-boost --demo
```

### 2. Profile a Real Cargo Project
Run `cargo-audit-boost` in the root of any Rust project. Pass standard cargo flags or commands as trailing arguments:
```bash
# Profile a cargo check build
cargo-audit-boost check

# Profile a release compilation
cargo-audit-boost build --release

# Profile target-specific builds
cargo-audit-boost test
```

---

## ⌨️ TUI Keyboard Shortcuts

| Shortcut | Description |
| :--- | :--- |
| `Up` / `Down` | Move selection cursor in the Gantt chart list |
| `PgUp` / `PgDn` | Scroll compiler diagnostics/logs in the details sidebar |
| `/` or `s` | Enter live search filter mode |
| `Esc` / `Enter` | Exit search mode |
| `c` / `C` | Toggle visibility of cached crates |
| `b` / `B` | Toggle bottleneck filter (shows top 3 slowest crates) |
| `q` / `Esc` | Quit the dashboard |

---

## 📂 Exported Output
After a scan completes, cargo-audit-boost generates performance reports in the target directory:
- **`target/build-scan/report.json`**: Raw serialized JSON compilation metrics database.
- **`target/build-scan/report.html`**: The premium browser-interactive dashboard report.

---

## 📄 License
This project is licensed under the MIT License. See [LICENSE](LICENSE) for details.
