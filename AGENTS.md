# AGENTS.md - EpochServer (Rust Port)

This document is for AI agents and human developers taking over work on the EpochServer Rust port.

## Project Overview

EpochServer is a custom Arma 3 extension (`callExtension`) originally written in C++ for Epoch Mod. It provides:

- Redis-backed persistence (the "hive")
- BattlEye RCon integration
- Steam Web API player validation
- Utility and anti-hack commands

The active codebase is this Rust project at the Git repository root. The goal is a drop-in replacement for the original extension's legacy string protocol, with identical SQF-facing output for existing Epoch server code.

Do not assume old C++ source directories such as `src/`, `msvs/`, or `deps/` exist in this checkout; the current `src/` directory is the Rust implementation.

## Repository Layout

```text
Repository root/
|-- .github/
|   |-- workflows/ci.yml
|   `-- workflows/release.yml
|-- Cargo.toml
|-- Cargo.lock
|-- build.rs
|-- EpochServer.example.ini
|-- docker-compose.redis.yml
|-- scripts/
|   |-- build-x64-release.ps1
|   |-- build-x64-release.sh
|   `-- epochserver_ingame_tests.sqf
|-- src/
|   |-- lib.rs          # Public exports + VERSION_STRING ("0.6.0.0")
|   |-- extension.rs    # RVExtension ABI + legacy command router
|   |-- redis.rs        # Async Redis layer
|   |-- be.rs           # BattlEye BERCon client
|   |-- steam.rs        # Steam Web API client
|   |-- config.rs       # INI loader matching original search order
|   |-- sqf.rs          # SQF array serialization
|   |-- modern.rs       # P3 structured API scaffold
|   `-- bin/tester.rs   # Standalone test harness
|-- tests/
|   |-- external_services.rs
|   |-- live_hive_commands.rs
|   `-- redis_integration.rs
|-- AGENTS.md
`-- README.md
```

## Versions And Naming

- Cargo package version: `0.7.0`
- Runtime compatibility/version string: `0.6.0.0` (`VERSION_STRING` in `src/lib.rs`)
- Canonical Windows artifact: `epochserver_x64.dll`
- Canonical Linux artifact: `libepochserver_x64.so`

Only the x64 variants are targeted for shipping. Non-x64 names such as `epochserver.dll` and `epochserver.so` are treated as stale/legacy artifacts.

## Essential Commands

Run these from the repository root.

Build:

```powershell
cargo build
cargo build --release
```

Fast unit/contract tests:

```powershell
cargo test --lib
```

List all discovered tests:

```powershell
cargo test -- --list
```

Redis-backed tests:

```powershell
docker compose -f docker-compose.redis.yml up -d
$env:EPOCH_REDIS_URL = "redis://127.0.0.1:6379/0"
cargo test --test redis_integration
```

Live extension tests:

```powershell
docker compose -f docker-compose.redis.yml up -d
$env:EPOCH_RUN_LIVE_REDIS_TESTS = "1"
$env:EPOCH_REDIS_URL = "redis://127.0.0.1:6379/0"
cargo test --test live_hive_commands -- --nocapture
```

Standalone tester:

```powershell
cargo run --release --bin tester -- "000" "500" "510" "810|5" "110|KEY:foo|0|[\"bar\",1]"
```

Clean x64-only helper builds:

```powershell
.\scripts\build-x64-release.ps1
```

```bash
./scripts/build-x64-release.sh
```

## Configuration Gotchas

1. Real config files are local.
   - Commit and ship `EpochServer.example.ini`.
   - Users copy it to `EpochServer.ini`.
   - `/EpochServer.ini` is ignored so local server configs are not committed or overwritten.

2. Official server check is on by default.
   - `OfficialCheck = 1` is the default.
   - If the check fails, `RVExtension` calls `std::process::exit(1)`.
   - For development, set `OfficialCheck = 0` in the loaded `EpochServer.ini` or set `EPOCHSERVER_OFFICIAL_CHECK=0`.

3. INI search order matches the original loader.
   - `{config_path}/EpochServer.ini`
   - `{profile_path}/EpochServer.ini`
   - `{config_path}/epochserver.ini`
   - `EPOCHSERVER_CONFIG_DIR` overrides the config directory and is especially useful for tests.

4. Redis URL environment variables take priority.
   - `EPOCH_REDIS_URL`
   - `REDIS_URL`
   - Then the `[Redis]` section in the INI.

5. SET and SETEX values must be valid JSON arrays.
   - The abuse filter intentionally rejects non-array values for compatibility.
   - Example good call: `110|PLAYER:123|0|["name","data",42]`

## Architecture Highlights

- `extension.rs` owns the raw C ABI exports and the legacy command router.
- `sqf.rs` is compatibility-sensitive. Small formatting changes can break existing SQF.
- Redis operations are async through Tokio and `redis::aio::ConnectionManager`.
- Pagination state for large GET/GETTTL results lives in `extension.rs`.
- `be.rs` contains the BERCon client plus test/mocking helpers.
- `steam.rs` contains Steam Web API calls for `001`.
- `modern.rs` is only scaffolding today; `RVExtensionArgs` currently delegates to the legacy router.

## Testing Philosophy

The project has a mix of pure unit tests, command contract tests, direct Redis integration tests, and live extension tests.

Current discovery from `cargo test -- --list` shows:

- 62 library unit/contract tests
- 18 live end-to-end extension tests in `tests/live_hive_commands.rs`
- 3 direct Redis integration tests in `tests/redis_integration.rs`
- 2 opt-in external-service smoke tests in `tests/external_services.rs`

`cargo test --lib` currently passes with 60 tests run and 2 ignored BE packet simulation tests.

`tests/live_hive_commands.rs` is the primary end-to-end regression suite. The tests are normal `#[test]` functions, but they are opt-in via `EPOCH_RUN_LIVE_REDIS_TESTS=1`; otherwise they skip early with a message.

`tests/redis_integration.rs` uses the `redis` crate directly against `EPOCH_REDIS_URL`, `REDIS_URL`, or `redis://127.0.0.1:6379/0`. It does not currently start testcontainers by itself, despite the dev-dependency still being present.

`tests/external_services.rs` covers real BattlEye RCon and Steam Web API smoke tests when the needed env vars are present. Without those env vars, it prints skip messages and returns successfully.

The last recorded local coverage notes put line coverage around 52% overall, with stronger coverage in isolated modules (`config.rs`, `redis.rs`, and `sqf.rs`). Rerun coverage before citing exact current percentages.

When modifying `extension.rs` or `sqf.rs`, run focused unit tests and then the live Redis suite when possible.

## Release Verification Notes

- Sync `800` and `820` now await their handlers inside the async router instead of nesting `runtime().block_on(...)`.
- CI clippy is enforced in `.github/workflows/ci.yml`.
- `.github/workflows/release.yml` uses the x64-only artifact names.
- Before release, run `tests/external_services.rs` with real BattlEye and Steam credentials when available.

## Current P3 Direction

- Keep the legacy string protocol 100% stable.
- Use `src/modern.rs` as a sandbox for future structured API work. Do not rewrite the legacy router around arma-rs yet. 
- Let `RVExtensionArgs` remain a compatibility delegate until a typed API is deliberately designed and tested.
- Keep `EPOCHSERVER_CONFIG_DIR` for hermetic structured API tests.

## Quick Handoff Checklist

1. Review `README.md` and this file for current packaging/config rules.
2. Run `cargo test --lib` for fast unit coverage.
3. Start Docker Redis and set `$env:EPOCH_REDIS_URL = "redis://127.0.0.1:6379/0"` for Redis-backed tests.
4. Run `cargo test --test redis_integration`.
5. Run live tests with `$env:EPOCH_RUN_LIVE_REDIS_TESTS = "1"` and `cargo test --test live_hive_commands -- --nocapture`.
6. For release verification, run `cargo test --test external_services -- --nocapture` with real `EPOCH_BE_*` and `EPOCH_STEAM_*` env vars if available.
7. Use `cargo run --release --bin tester -- ...` for quick manual command exploration.
8. Before release, ensure only `EpochServer.example.ini` is packaged, never a live `EpochServer.ini`.

## Useful References

- `README.md` - User-facing docs and current status
- `tests/live_hive_commands.rs` - Primary end-to-end compatibility suite
- `tests/external_services.rs` - Optional real BE/Steam smoke tests
- `src/extension.rs` - Legacy command behavior source of truth
- `.github/workflows/ci.yml` - CI build/test workflow
- `.github/workflows/release.yml` - Release packaging workflow

Maintain this file whenever architecture, testing strategy, packaging rules, or major limitations change.
