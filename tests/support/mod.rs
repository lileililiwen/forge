//! Test harness modules.
//!
//! Suites that need one world (the share suites) include
//! `support/share.rs` directly. Suites that need two — interest
//! imports build on the same identity and registry fixture the share
//! suites established — include this file instead, which exposes both
//! as submodules.

#![allow(dead_code)]

pub mod interest;
pub mod share;
