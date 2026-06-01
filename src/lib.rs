//! EpochServer — modern Rust reimplementation of the Arma 3 Epoch hive extension.
//!
//! This crate produces a `cdylib` that Arma 3 loads via `callExtension`.
//! Full behavioral compatibility with the original C++ `epochserver` is the #1 goal.

#![deny(rust_2018_idioms)]
// #![warn(missing_docs)] // enable later

pub mod be; // BattlEye BERCon client
pub mod config; // INI loader matching original search order and defaults
pub mod extension; // Raw RVExtension exports + command handlers
pub mod redis; // Async Redis layer (tokio + connection manager)
pub mod sqf; // Exact SQF array serializer (highest-risk piece — done)
pub mod steam; // Steam Web API client (for 001) - in progress

pub mod modern; // P3: Structured / arma-rs based API (scaffolding started)

// Future modules (per approved plan):
// pub mod commands;
// pub mod redis;
// pub mod be;
// pub mod steam;
// pub mod config;
// pub mod epochlib;

// Re-export the SQF types for convenience during early development.
pub use sqf::{SQFValue, SQF};

/// Version string returned on empty call.
/// We return something close to the original for compatibility and testing.
pub const VERSION_STRING: &str = "0.6.0.0";

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanity() {
        assert!(!VERSION_STRING.is_empty());
    }
}

/// Internal contract tests for the handlers (these will grow into the full
/// compatibility harness once we have more commands + the external tester).
#[cfg(test)]
mod contract_tests {
    use super::extension::{handle_000, handle_510};
    use super::VERSION_STRING;

    #[test]
    fn historical_000_and_510_contract() {
        let out_000 = handle_000();
        assert!(
            out_000.starts_with('[') && (out_000.contains("NA123") || out_000.contains("TEST01"))
        );

        let out_510 = handle_510();
        assert!(out_510.matches(',').count() == 5 && out_510.starts_with('['));

        assert!(!VERSION_STRING.is_empty());
    }
}
