# Publishing Guide: Uploading `cargo-sonar-boost` to Crates.io 📦

This guide covers the necessary steps to prepare, validate, and publish `cargo-sonar-boost` to the official Rust package registry (crates.io).

---

## Step 1: Set Up your Crates.io Account

1.  Navigate to [crates.io](https://crates.io) and log in using your GitHub account.
2.  Go to your **Account Settings** -> **API Tokens**.
3.  Click **New Token**, select scopes (you need `publish-update` or `publish-new`), give it a name (e.g. `sonar-boost-publishing`), and click **Generate Token**.
4.  Copy the generated token immediately (it will only be shown once).

---

## Step 2: Login Locally

Run the following command in your terminal, pasting your copied API token when prompted:
```bash
cargo login
```
This saves your token locally in `~/.cargo/credentials.toml` so cargo can authenticate you for publishing.

---

## Step 3: Update Crate Metadata in `Cargo.toml`

Crates.io rejects uploads if they lack essential package metadata fields. Open [`Cargo.toml`](file:///home/paul/code/cargo-sonar/Cargo.toml) and ensure the following fields are defined under `[package]`:

```toml
[package]
name = "cargo-sonar-boost"
version = "0.1.0"
edition = "2024"
authors = ["Your Name <email@example.com>"]
description = "🛡️ A zero-config code quality and technical debt dashboard CLI for Rust"
license = "MIT"
readme = "README.md"
repository = "https://github.com/lileililiwen/cargo-sonar-boost"
keywords = ["cargo", "clippy", "quality", "dashboard", "deny"]
categories = ["development-tools::cargo-plugins", "development-tools"]
```

---

## Step 4: Perform a Dry-Run Verification

Before uploading, verify that cargo can package and compile your project in a clean environment:
```bash
cargo publish --dry-run
```
This command bundles the crate, extracts it to target, compiles it, and validates files. Fix any warnings or missing file inclusions (e.g., ensure `README.md` and `LICENSE` are packaged).

---

## Step 5: Publish Crate

Once the dry-run compiles without errors, publish your package to crates.io:
```bash
cargo publish
```
This uploads the cargo-sonar-boost package. Within a few minutes:
- Your package page will be live at `https://crates.io/crates/cargo-sonar-boost`.
- Any developer can install it using:
  ```bash
  cargo install cargo-sonar-boost
  ```
- It will automatically index and generate rustdoc documentation at `https://docs.rs/cargo-sonar-boost`.
