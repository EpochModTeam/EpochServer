//! Build script to enforce our "x64 only" artifact naming policy.
//!
//! We deliberately only produce and ship the `_x64` variants of the extension
//! (epochserver_x64.dll / libepochserver_x64.so). This matches the long-standing
//! Arma 3 convention where servers use `callExtension "epochserver_x64"`.
//!
//! The main control is in Cargo.toml: [lib] name = "epochserver_x64".
//! This script provides an additional hint for the MSVC linker.
//! Any remaining non-x64 files are explicitly deleted in CI before upload.

fn main() {
    // The [lib] name in Cargo.toml is the source of truth for artifact naming.
    // Do not pass /OUT here: it makes MSVC write the DLL into the workspace root
    // and leaves Cargo's target/release artifact stale.
    println!("cargo:rerun-if-changed=build.rs");
}
