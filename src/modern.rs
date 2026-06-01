//! P3: Modern / structured API layer using `arma-rs`.
//!
//! Goal: Offer a clean, typed command interface for new code while
//! preserving 100% backward compatibility with the existing string-based
//! `callExtension` protocol used by all current Epoch SQF.
//!
//! Current status:
//! - The legacy `RVExtension` path remains the primary entry point and must not regress.
//! - `RVExtensionArgs` currently delegates to that same legacy router so it never returns
//!   a fake or diagnostic-only response.
//! - Structured command definitions can be added here incrementally without changing
//!   the existing string protocol.

pub fn init() {
    // Reserved for future arma-rs registration.
}
