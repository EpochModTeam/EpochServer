# EpochServer (Rust)

Modern Rust reimplementation of the Epoch Mod Arma 3 server extension.

This crate builds a `cdylib` that Arma 3 loads via `callExtension`. It provides Redis-backed persistence, BattlEye RCon integration, Steam Web API player validation, and utility/anti-hack commands used by Epoch servers.

The goal is a drop-in replacement for the original C++ `epochserver` protocol while keeping the legacy SQF-facing string API stable.

## Status

The current Rust port implements the legacy `RVExtension`, `RVExtensionVersion`, and `RVExtensionArgs` exports. `RVExtensionArgs` currently delegates to the same legacy router; structured `arma-rs` commands are scaffolded in `src/modern.rs` but not yet a separate public API.

The Cargo package version is `0.7.0`. The extension compatibility string returned by `RVExtensionVersion` is still `0.6.0.0` via `VERSION_STRING` in `src/lib.rs`.

Current command coverage includes the known Epoch command families routed in `src/extension.rs`: `000`, `001`, `100`/`T100`, `110`/`111`, `120`/`121`, `130`/`131`, `140`/`141`, `200`, `210`, `220`, `240`, `250`, `300`, `400`, `500`, `510`, `600`, `700`/`701`, `800`/`801`, `810`, `820`/`821`, `830`, `840`, and the BattlEye `9xx` commands currently used by the port.

| Area | Current state | Automated coverage |
| --- | --- | --- |
| Redis hive commands | Implemented | Strong live coverage in `tests/live_hive_commands.rs` |
| SQF serialization | Implemented and compatibility-sensitive | Unit coverage |
| BattlEye RCon | Implemented | Mock/graceful-path coverage plus opt-in real RCon smoke test |
| Steam Web API `001` checks | Implemented | Unit coverage plus opt-in real Steam API smoke test |
| File anti-hack `800`/`820` | Implemented | Temp-directory live coverage |
| Official server MD5 check | Implemented | Development bypass via config/env |

Remaining polish is mostly around running the opt-in real BattlEye/Steam smoke tests before release when suitable credentials are available.

## Requirements

- Arma 3 server, Windows or Linux
- Redis 6+ for hive persistence
- Rust toolchain, only required when building from source
- Docker, optional but recommended for local Redis testing

## Build

Run commands from this `EpochServer/` directory.

Windows:

```powershell
cargo build --release
# target\release\epochserver_x64.dll
```

Linux:

```bash
cargo build --release
# target/release/libepochserver_x64.so
```

Only x64 extension names are targeted and shipped:

- Windows: `epochserver_x64.dll`
- Linux: `libepochserver_x64.so`

Non-x64 names such as `epochserver.dll` and `epochserver.so` are intentionally ignored.

For clean local builds that also remove stale non-x64 artifacts:

```powershell
.\scripts\build-x64-release.ps1
```

```bash
./scripts/build-x64-release.sh
```

CI workflows live in `.github/workflows/`. The main CI workflow builds and tests the Rust project on Windows and Linux.

## Configure

Commit and ship `EpochServer.example.ini`, not a live `EpochServer.ini`.

For a server install, copy `EpochServer.example.ini` to `EpochServer.ini` next to `epochserver_x64.dll` / `libepochserver_x64.so`, or into the configured profile directory. Edit the Redis settings at minimum.

Release archives should include `EpochServer.example.ini`, not `EpochServer.ini`, so upgrades do not overwrite existing server configs.

The runtime loader searches in the original order:

1. `{config_path}/EpochServer.ini`
2. `{profile_path}/EpochServer.ini`
3. `{config_path}/epochserver.ini`

You can force a specific config directory with `EPOCHSERVER_CONFIG_DIR`, mainly for tests and advanced deployment.

For local development without an official `a3_epoch_server.pbo`, set `OfficialCheck = 0` in the `EpochServer.ini` being loaded, or set `EPOCHSERVER_OFFICIAL_CHECK=0`.

## Install

Place the built extension in your Arma 3 server extension folder, usually the folder referenced by your Epoch `callExtension` setup, and restart the server.

SQF calls should use the x64 extension name:

```sqf
"epochserver_x64" callExtension "000";
"epochserver_x64" callExtension "500";
"epochserver_x64" callExtension format ["110|%1:%2|%3|%4", _prefix, _key, _hiveCallID, _value];
```

## Testing

Fast unit tests:

```powershell
cargo test --lib
```

List all discovered tests:

```powershell
cargo test -- --list
```

As currently listed, the project has 62 library unit/contract tests, 18 live extension tests, 3 direct Redis integration tests, and 2 opt-in external-service smoke tests. `cargo test --lib` currently passes with 60 run and 2 ignored BE packet simulation tests.

Start Redis for Redis-backed tests:

```powershell
docker compose -f docker-compose.redis.yml up -d
$env:EPOCH_REDIS_URL = "redis://127.0.0.1:6379/0"
```

Linux/macOS:

```bash
docker compose -f docker-compose.redis.yml up -d
export EPOCH_REDIS_URL="redis://127.0.0.1:6379/0"
```

Direct Redis integration tests:

```powershell
cargo test --test redis_integration
```

These use the `redis` crate directly against `EPOCH_REDIS_URL`, `REDIS_URL`, or `redis://127.0.0.1:6379/0`. They verify Redis connectivity, SET/GET roundtrips, large value SET/GET, and INCR counters. They do not currently start testcontainers automatically.

Live end-to-end `callExtension` tests are opt-in. Without `EPOCH_RUN_LIVE_REDIS_TESTS=1`, they print a skip message and return early.

```powershell
$env:EPOCH_RUN_LIVE_REDIS_TESTS = "1"
$env:EPOCH_REDIS_URL = "redis://127.0.0.1:6379/0"
cargo test --test live_hive_commands -- --nocapture
```

Use `cargo test --lib` for fast Redis-free unit coverage. Plain `cargo test` also discovers `tests/redis_integration.rs`, which requires a reachable Redis instance.

Real BattlEye and Steam smoke tests are also opt-in:

```powershell
$env:EPOCH_BE_IP = "127.0.0.1"
$env:EPOCH_BE_PORT = "2306"
$env:EPOCH_BE_PASSWORD = "your-rcon-password"
$env:EPOCH_STEAM_API_KEY = "your-steam-web-api-key"
$env:EPOCH_STEAM_TEST_ID = "76561197960435530"
cargo test --test external_services -- --nocapture
```

Standalone tester:

```powershell
cargo run --release --bin tester -- "000" "510" "500" "810|3"
```

For the most realistic manual testing, run the standalone tester against a persistent local Redis while exercising real SQF `callExtension` strings.

## Project Layout

```text
EpochServer/
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
|   |-- lib.rs
|   |-- extension.rs
|   |-- redis.rs
|   |-- be.rs
|   |-- steam.rs
|   |-- config.rs
|   |-- sqf.rs
|   |-- modern.rs
|   `-- bin/tester.rs
|-- tests/
|   |-- external_services.rs
|   |-- live_hive_commands.rs
|   `-- redis_integration.rs
|-- AGENTS.md
`-- README.md
```

## License

APL-SA (Arma Public License - Share Alike)

## Credits

Original C++ EpochServer:

- Aaron Clark - [VB]AWOL
- Florian Kinder - Fank
- Denis Erygin - devd

Rust port:

- Epoch Mod Team
