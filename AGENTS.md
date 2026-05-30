# AGENTS.md — EpochServer (Rust Port)

This document is for AI agents and human developers taking over work on the EpochServer project.

## Project Overview

**EpochServer** is a custom Arma 3 extension (`callExtension`) originally written in C++ for the Epoch Mod. It provides:

- Redis-backed persistence (the "hive")
- BattlEye RCon integration
- Steam Web API player validation
- Various utility and anti-hack functions

The **active codebase** is a modern Rust reimplementation located in the `EpochServer/` subdirectory. The goal is a **drop-in replacement** that returns identical output to the original C++ version for all existing SQF code.

**Important**: The original C++ source remains at the repository root (`src/`, `msvs/`, `deps/`, etc.) purely for historical reference. **All new development happens inside `EpochServer/`.**

## Current Directory Structure (Active Work)

```
EpochServer/                  ← ACTIVE RUST PROJECT (cd here for everything)
├── Cargo.toml
├── src/
│   ├── lib.rs                # Public exports + VERSION_STRING ("0.6.0.0")
│   ├── extension.rs          # Core: RVExtension + all command handlers (110, 200, 700, 9xx, etc.)
│   ├── redis.rs              # Async Redis layer (Tokio + connection manager)
│   ├── be.rs                 # BattlEye BERCon client
│   ├── steam.rs              # Steam Web API client (001 checks)
│   ├── config.rs             # INI loader (exact original search order)
│   ├── sqf.rs                # SQF array serialization (critical fidelity)
│   └── bin/tester.rs         # Standalone test harness (loads the DLL and calls RVExtension)
├── tests/
│   ├── live_hive_commands.rs # ★ THE IMPORTANT ONE — comprehensive live Redis tests
│   └── redis_integration.rs  # Lower-level direct redis crate tests (+ testcontainers)
├── docker-compose.redis.yml
├── EpochServer.ini           # Sample config (copy & edit for real use)
├── PARITY.md                 # Command-by-command compatibility status
├── TESTING.md                # How to run tests and the tester
├── README.md                 # User-facing docs
└── .github/workflows/ci.yml  # Multi-platform build + test CI

# At repository root (mostly legacy — do not modify for active work)
src/          # Original C++ source
msvs/         # Old Visual Studio solution
deps/         # Old C++ dependencies (pcre, rapidjson, etc.)
sqf/          # SQF side (also copied under EpochServer/sqf/)
```

## Essential Commands (Always run from inside `EpochServer/`)

### Build
```powershell
# Debug (fast, for development)
cargo build

# Release (the actual DLL you ship — optimized + stripped)
cargo build --release
# Output: target/release/epochserver.dll
```

### Running Tests
```powershell
# Unit + doc tests (fast)
cargo test

# The comprehensive live command tests (recommended for verification)
$env:EPOCH_REDIS_URL = "redis://127.0.0.1:6379/0"
cargo test --test live_hive_commands -- --nocapture
```

### Live Redis (Docker)
```powershell
docker compose -f docker-compose.redis.yml up -d
$env:EPOCH_REDIS_URL = "redis://127.0.0.1:6379/0"
```

### Using the Standalone Tester (very useful)
```powershell
cargo run --release --bin tester -- "000" "500" "510" "810|5" "110|KEY:foo|0|[\"bar\",1]"
```

## Critical Configuration Gotchas

1. **Official Server Check** (`OfficialCheck = 1` by default)
   - The extension will call `std::process::exit(1)` if it cannot find a valid `a3_epoch_server.pbo` with the correct MD5.
   - For development and testing, you **must** set `OfficialCheck = 0` in the `EpochServer.ini` that the DLL finds.
   - The `live_hive_commands.rs` tests automatically create suitable INI files in `target/debug/`, `target/debug/deps/`, etc.

2. **INI Search Order** (matches original exactly)
   - `{config_path}/EpochServer.ini` (where the DLL is loaded from)
   - `{profile_path}/EpochServer.ini`
   - `{config_path}/epochserver.ini` (lowercase fallback)

3. **SET / SETEX require valid JSON arrays**
   - The abuse filter rejects non-array values (this is intentional fidelity).
   - Example good call: `110|PLAYER:123|0|["name","data",42]`

4. **Redis URL**
   - The extension respects `EPOCH_REDIS_URL` and `REDIS_URL` (highest priority).

## Code Architecture Highlights

- `extension.rs` contains the giant `handle_command` match + all the `handle_xxx` functions. This is where most behavioral work happens.
- `sqf.rs` is extremely sensitive — small changes here break compatibility with existing SQF.
- Pagination state for large GET results (the old "tempGet" hack) lives in `extension.rs`.
- Redis operations are async via Tokio; the extension uses a global runtime.
- BattlEye and Steam commands are mostly fire-and-forget (spawned tasks).

## Testing Philosophy

- **Unit tests** in `src/` are minimal.
- **`tests/live_hive_commands.rs`** is the primary regression suite. It loads the real DLL and drives realistic `callExtension` strings against live Redis. Add new command coverage here.
- The older `redis_integration.rs` uses the `redis` crate directly + testcontainers (good for low-level Redis layer testing).

When adding or fixing commands, always update the corresponding test in `live_hive_commands.rs` and run it with a real Redis instance.

## Current Known Issues / Technical Debt (as of handoff)

- Compiler warnings:
  - Multiple `extern "stdcall"` (should be `extern "system"` for cross-platform)
  - Deprecated `redis::Client::get_tokio_connection_manager` (use `get_connection_manager`)
  - Various unused imports/variables/dead code
- The tester binary can panic on rapid successive calls due to Tokio runtime nesting (not a problem in real Arma usage).
- Some edge cases in async logging + abuse reporting still surface under heavy tester use.
- CI runs `cargo clippy --release -- -D warnings` with `continue-on-error: true` — clean this up over time.

## Recent Handoff Context (What Was Just Done)

- Project was restructured so the Rust implementation lives cleanly under `EpochServer/`.
- Legacy C++ remains at root for reference only.
- A full set of live end-to-end command tests was added (`live_hive_commands.rs` — 13 tests covering 000, 110/200, 120/210, 130/300, 400, 500, 510, 600, 700, 810, 830, 220, 240/140, etc.).
- All tests pass against live Redis.
- Build (debug + release) and basic test flow is stable on Windows.

## Quick Handoff Checklist for Next Agent

1. `cd EpochServer`
2. Make sure Docker Redis is running + `$env:EPOCH_REDIS_URL` is set.
3. `cargo build --release`
4. `cargo test --test live_hive_commands -- --nocapture` (this should be your primary verification command).
5. Use the `tester` binary for quick manual exploration of any command.
6. Read `PARITY.md` before touching any command handler.
7. When modifying `extension.rs` or `sqf.rs`, run the live tests immediately.

## Useful References Inside the Repo

- `EpochServer/PARITY.md` — Detailed command status table
- `EpochServer/TESTING.md` — Original testing instructions
- `EpochServer/README.md` — High-level user docs
- Root `README.md` — Historical C++ call documentation (still useful as the canonical list of expected behaviors)

---

**Maintain this file.** Update it whenever architecture, testing strategy, or major limitations change. It is the primary handoff document for future agents.