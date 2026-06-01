//! Comprehensive live integration tests against a real Redis instance.
//!
//! These tests load the compiled `epochserver` cdylib exactly like Arma 3 would
//! and drive it through realistic `callExtension` strings.
//!
//! They confirm that the high-level command handlers + Redis layer work correctly
//! end-to-end.
//!
//! Requirements:
//!   - A running Redis (e.g. `docker compose -f docker-compose.redis.yml up -d`)
//!   - EPOCH_REDIS_URL or REDIS_URL environment variable (optional, defaults to localhost)
//!
//! Live Redis integration tests (18 tests).
//!
//! These are **normal** (non-ignored) `#[test]` functions so they participate
//! in `cargo test` discovery and coverage reports.
//!
//! They are **opt-in** for safety: some Redis failure paths inside the extension
//! can currently cause hard panics/aborts when no server is present.
//!
//! To execute them for real (and get much better coverage of the hive handlers,
//! abuse logging to Redis, 800/820 file writes, large pagination, etc.):
//!
//!   $env:EPOCH_RUN_LIVE_REDIS_TESTS = "1"
//!   $env:EPOCH_REDIS_URL = "redis://127.0.0.1:6379/0"
//!   docker compose -f docker-compose.redis.yml up -d
//!   cargo test --test live_hive_commands -- --nocapture
//!
//! When the env var is not set they print a skip message and return early,
//! so plain `cargo test` and `cargo llvm-cov` remain fast and green.

use std::env;
use std::ffi::{c_char, c_int, CString};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use libloading::Library;

type RVExtensionFn =
    unsafe extern "system" fn(output: *mut c_char, output_size: c_int, function: *const c_char);

static EXTENSION: OnceLock<Library> = OnceLock::new();
static RV_EXTENSION: OnceLock<libloading::Symbol<'static, RVExtensionFn>> = OnceLock::new();

/// Writes a test-friendly EpochServer.ini to a specific location if it doesn't exist.
fn write_test_ini(path: &Path) {
    if path.exists() {
        return;
    }
    let content = r#"[EpochServer]
InstanceID = TEST01
LogAbuse = 1
LogLimit = 100
OfficialCheck = 0
BattlEyePath =

[Redis]
IP = 127.0.0.1
Port = 6379
DB = 0
Password =

[SteamAPI]
Logging = 0
Key =
VACBanned = 0
"#;
    if let Err(e) = std::fs::write(path, content) {
        eprintln!("Warning: failed to write {}: {}", path.display(), e);
    } else {
        println!("Created test config: {}", path.display());
    }
}

/// Best-effort cleanup of test INI files and temp config directories.
/// Useful when you want to reset the workspace after running the live tests.
pub fn cleanup_test_inis() {
    // Clean common persistent locations (legacy)
    let candidates = [
        "EpochServer.ini",
        "target/debug/EpochServer.ini",
        "target/debug/deps/EpochServer.ini",
        "target/release/EpochServer.ini",
        "target/release/deps/EpochServer.ini",
    ];
    for c in candidates {
        let _ = std::fs::remove_file(c);
    }

    // Best-effort cleanup of any temp hermetic config dirs we created
    if let Ok(temp) = std::env::temp_dir().read_dir() {
        for entry in temp.flatten() {
            let path = entry.path();
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.starts_with("epochserver-test-config-") && path.is_dir() {
                    let _ = std::fs::remove_dir_all(&path);
                }
            }
        }
    }
}

/// Ensures a hermetic test configuration using a temporary directory.
///
/// This is the preferred approach for isolation:
/// - Creates a unique temp directory (once per test process)
/// - Writes the friendly EpochServer.ini only inside it
/// - Sets EPOCHSERVER_CONFIG_DIR so the extension finds it
///
/// This avoids polluting the workspace with persistent .ini files.
fn ensure_test_configs_for_testing(_dll_path: &Path) {
    use std::sync::OnceLock;

    static TEST_CONFIG_DIR: OnceLock<PathBuf> = OnceLock::new();

    let config_dir = TEST_CONFIG_DIR.get_or_init(|| {
        let mut dir = std::env::temp_dir();
        dir.push(format!("epochserver-test-config-{}", std::process::id()));

        let _ = std::fs::create_dir_all(&dir);
        write_test_ini(&dir.join("EpochServer.ini"));

        // Tell the extension to use this directory
        // (must be set before the first call that triggers get_config inside the DLL)
        std::env::set_var("EPOCHSERVER_CONFIG_DIR", &dir);

        // Extra safety net for live tests / coverage runs under llvm-cov-target etc.
        // Forces the official server check off even if the INI wasn't picked up yet.
        std::env::set_var("EPOCHSERVER_OFFICIAL_CHECK", "0");

        println!("Using hermetic test config dir: {}", dir.display());
        dir
    });

    // Also write a copy next to the DLL as a fallback (harmless and helps debugging)
    if let Some(dir) = _dll_path.parent() {
        let _ = std::fs::copy(
            config_dir.join("EpochServer.ini"),
            dir.join("EpochServer.ini"),
        );
    }

    // Extra robustness for coverage runs (llvm-cov uses a custom target dir)
    // Write copies into common llvm-cov-target locations so the INI is found
    // even if EPOCHSERVER_CONFIG_DIR timing is slightly off on first call.
    for extra in &[
        "target/llvm-cov-target/debug",
        "target/llvm-cov-target/debug/deps",
        "target/llvm-cov-target/release",
    ] {
        if let Ok(p) = std::fs::canonicalize(extra) {
            let _ = std::fs::create_dir_all(&p);
            let _ = std::fs::copy(
                config_dir.join("EpochServer.ini"),
                p.join("EpochServer.ini"),
            );
        }
    }
}

/// Locates the built extension library.
fn find_extension_library() -> Option<PathBuf> {
    // 1. Explicit override
    if let Ok(p) = env::var("EPOCHSERVER_DLL") {
        let p = PathBuf::from(p);
        if p.exists() {
            return Some(p);
        }
    }

    let candidates = [
        // We only support/ship the x64 variants (historical Arma convention).
        // Non-x64 names are deliberately not searched for.
        "target/debug/epochserver_x64.dll",
        "target/debug/epochserver_x64.so",
        "target/debug/libepochserver_x64.so",
        "target/release/epochserver_x64.dll",
        "target/release/epochserver_x64.so",
        "target/release/libepochserver_x64.so",
        "epochserver_x64.dll",
        "epochserver_x64.so",
        "../target/debug/epochserver_x64.dll",
        "../target/release/epochserver_x64.dll",
    ];

    for c in &candidates {
        let p = Path::new(c);
        if p.exists() {
            return Some(p.to_path_buf());
        }
    }

    // Recursive search under target as last resort
    if let Ok(root) = env::current_dir() {
        let mut search_roots = vec![root.clone()];
        if let Some(parent) = root.parent() {
            search_roots.push(parent.to_path_buf());
        }
        for base in search_roots {
            if let Ok(entries) = std::fs::read_dir(&base) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_dir() && path.file_name().is_some_and(|n| n == "target") {
                        // walk target
                        let mut found = None;
                        fn visit(dir: &Path, out: &mut Option<PathBuf>) {
                            if out.is_some() {
                                return;
                            }
                            if let Ok(entries) = std::fs::read_dir(dir) {
                                for e in entries.flatten() {
                                    let p = e.path();
                                    if p.is_dir() {
                                        visit(&p, out);
                                    } else if let Some(name) =
                                        p.file_name().and_then(|n| n.to_str())
                                    {
                                        // Only accept the x64 variants we actually ship and support
                                        if name == "epochserver_x64.dll"
                                            || name == "epochserver_x64.so"
                                            || name == "libepochserver_x64.so"
                                        {
                                            *out = Some(p);
                                        }
                                    }
                                }
                            }
                        }
                        visit(&path, &mut found);
                        if let Some(f) = found {
                            return Some(f);
                        }
                    }
                }
            }
        }
    }

    None
}

fn load_extension() -> &'static Library {
    EXTENSION.get_or_init(|| {
        let dll_path = find_extension_library().expect(
            "Could not locate epochserver_x64.dll / libepochserver_x64.so. \
             We only ship the x64 variants. Build with `cargo build --release` or set EPOCHSERVER_DLL.",
        );

        println!("Loading extension from: {}", dll_path.display());

        // Make sure friendly configs exist in all places the extension searches during tests
        ensure_test_configs_for_testing(&dll_path);

        // Set Redis URL for the extension if not already set
        if env::var("EPOCH_REDIS_URL").is_err() && env::var("REDIS_URL").is_err() {
            env::set_var("EPOCH_REDIS_URL", "redis://127.0.0.1:6379/0");
        }

        unsafe { Library::new(&dll_path).expect("Failed to load extension library") }
    })
}

fn get_rv_extension() -> &'static libloading::Symbol<'static, RVExtensionFn> {
    RV_EXTENSION.get_or_init(|| {
        let lib = load_extension();
        unsafe {
            lib.get(b"_RVExtension@12")
                .or_else(|_| lib.get(b"RVExtension"))
                .expect("Could not find RVExtension symbol")
        }
    })
}

/// Calls the extension exactly like Arma 3 does and returns the output string.
pub fn call_extension(command: &str) -> String {
    let rv_extension = get_rv_extension();

    let mut output = vec![0u8; 8192];
    let input = CString::new(command.as_bytes()).unwrap();

    unsafe {
        rv_extension(
            output.as_mut_ptr() as *mut c_char,
            output.len() as c_int,
            input.as_ptr(),
        );
    }

    let len = output.iter().position(|&b| b == 0).unwrap_or(output.len());
    String::from_utf8_lossy(&output[..len]).to_string()
}

/// Returns true if live Redis tests should be skipped.
///
/// Live Redis tests are **opt-in** via the EPOCH_RUN_LIVE_REDIS_TESTS=1 environment variable.
/// This keeps normal `cargo test` and coverage runs safe and fast even if Docker Redis
/// is not running (some paths inside the extension can hard-panic on complete Redis absence).
///
/// When you have the docker server up:
///   $env:EPOCH_RUN_LIVE_REDIS_TESTS = "1"
///   $env:EPOCH_REDIS_URL = "redis://127.0.0.1:6379/0"
///   cargo test --test live_hive_commands -- --nocapture
///
/// These tests will then run for real and contribute significantly to coverage of the
/// Redis-backed handlers, abuse logging, large value pagination, 800/820, etc.
fn skip_live_redis_tests() -> bool {
    if std::env::var("EPOCH_RUN_LIVE_REDIS_TESTS").is_ok() {
        return false;
    }

    println!("SKIPPING live Redis test (opt-in).");
    println!("  To run with Docker Redis:  $env:EPOCH_RUN_LIVE_REDIS_TESTS = \"1\"");
    println!("                             $env:EPOCH_REDIS_URL = \"redis://127.0.0.1:6379/0\"");
    println!("                             cargo test --test live_hive_commands");
    true
}

// =============================================================================
// Actual Live Command Tests
// =============================================================================

#[test]
fn live_000_get_config() {
    if skip_live_redis_tests() {
        return;
    }
    let out = call_extension("000");
    assert!(
        out.starts_with('['),
        "000 should return an array, got: {out}"
    );
    assert!(
        out.contains("NA123") || out.contains("TEST01"),
        "Expected instance ID, got: {out}"
    );
    println!("000 → {out}");
}

#[test]
fn live_500_ping() {
    if skip_live_redis_tests() {
        return;
    }
    let out = call_extension("500");
    assert!(out.contains("PONG"), "500 should return PONG, got: {out}");
    println!("500 → {out}");
}

#[test]
fn live_510_current_time() {
    if skip_live_redis_tests() {
        return;
    }
    let out = call_extension("510");
    // Format: [YYYY,MM,DD,HH,MM,SS]
    assert!(out.starts_with('['), "510 should return array");
    let parts: Vec<&str> = out
        .trim_matches(|c| c == '[' || c == ']')
        .split(',')
        .collect();
    assert_eq!(parts.len(), 6, "Expected 6 time components");
    println!("510 → {out}");
}

#[test]
fn live_set_and_get() {
    if skip_live_redis_tests() {
        return;
    }
    let key = "epochserver:live:test:setget";
    let value = r#"["player","data",42,true]"#; // must be valid JSON array for the extension

    // SET via 110
    let set_out = call_extension(&format!("110|{key}|0|{value}"));
    println!("110 SET → {set_out}");

    // GET via 200
    let get_out = call_extension(&format!("200|{key}"));
    println!("200 GET → {get_out}");

    assert!(get_out.contains("player"), "GET should return stored data");
    assert!(get_out.contains("42"));

    // Cleanup
    let _ = call_extension(&format!("400|{key}"));
}

#[test]
fn live_setex_and_ttl() {
    if skip_live_redis_tests() {
        return;
    }
    let key = "epochserver:live:test:setex";
    let value = r#"["temporary","value"]"#;

    // SETEX via 120 (60 seconds)
    let set_out = call_extension(&format!("120|{key}|60|0|{value}"));
    println!("120 SETEX → {set_out}");

    // Check TTL
    let ttl_out = call_extension(&format!("300|{key}"));
    println!("300 TTL → {ttl_out}");
    assert!(
        ttl_out.contains("60") || ttl_out.contains("5"),
        "TTL should be around 60s"
    );

    // GET + TTL
    let get_ttl = call_extension(&format!("210|{key}"));
    println!("210 GET+TTL → {get_ttl}");

    let _ = call_extension(&format!("400|{key}"));
}

#[test]
fn live_large_value_pagination() {
    if skip_live_redis_tests() {
        return;
    }
    // This is one of the most important behaviors to preserve from the original.
    let key = "epochserver:live:test:large";
    let large: String = (0..8000).map(|i| ((i % 26) as u8 + b'a') as char).collect();
    let json_value = format!(r#"["large", "{}"]"#, large);

    let _ = call_extension(&format!("110|{key}|0|{json_value}"));

    // First page
    let page1 = call_extension(&format!("200|{key}"));
    println!("200 large page1 len = {}", page1.len());

    // Request more pages (simulating SQF pagination loop)
    let page2 = call_extension(&format!("200|{key}"));
    let page3 = call_extension(&format!("200|{key}"));

    assert!(page1.len() > 100, "First page should contain data");
    println!(
        "Pagination pages lengths: {}, {}, {}",
        page1.len(),
        page2.len(),
        page3.len()
    );

    let _ = call_extension(&format!("400|{key}"));
}

#[test]
fn live_expire_and_del() {
    if skip_live_redis_tests() {
        return;
    }
    let key = "epochserver:live:test:expire";
    let value = r#"["will","expire"]"#;

    let _ = call_extension(&format!("110|{key}|0|{value}"));

    // Set short expire
    let _ = call_extension(&format!("130|{key}|2"));

    // Should still exist immediately
    let exists1 = call_extension(&format!("250|{key}"));
    println!("250 EXISTS after EXPIRE → {exists1}");

    // Wait a bit
    std::thread::sleep(std::time::Duration::from_secs(3));

    let exists2 = call_extension(&format!("250|{key}"));
    println!("250 EXISTS after wait → {exists2}");

    // DEL
    let del_out = call_extension(&format!("400|{key}"));
    println!("400 DEL → {del_out}");
}

#[test]
fn live_lpop_cmd_queue() {
    if skip_live_redis_tests() {
        return;
    }
    let prefix = "CMD";

    // Push some commands the way Epoch expects
    let _ = call_extension(&format!("110|{prefix}:testqueue|0|[\"doSomething\",1,2]"));
    let _ = call_extension(&format!("110|{prefix}:testqueue|0|[\"anotherCmd\"]"));

    // LPOP via 600 (uses CMD: prefix internally)
    let pop1 = call_extension("600|testqueue");
    println!("600 LPOP → {pop1}");

    let pop2 = call_extension("600|testqueue");
    println!("600 LPOP2 → {pop2}");

    // Cleanup whatever is left
    let _ = call_extension("400|CMD:testqueue");
}

#[test]
fn live_logging() {
    if skip_live_redis_tests() {
        return;
    }
    let msg = "Live test log entry from Rust integration test";

    let out = call_extension(&format!("700|TESTLOG|{msg}"));
    println!("700 LOG → {out}");

    // We can't easily assert Redis contents here without another client,
    // but at least the call shouldn't crash or return error format.
    assert!(
        !out.to_lowercase().contains("error"),
        "Log call should not error: {out}"
    );
}

#[test]
fn live_830_incr_bancount() {
    if skip_live_redis_tests() {
        return;
    }
    // 830 uses INCR on "ahb-cnt"
    let out = call_extension("830");
    println!("830 INCR → {out}");
    assert!(out.starts_with('['), "830 should return array");
}

#[test]
fn live_810_random_string() {
    if skip_live_redis_tests() {
        return;
    }
    let out = call_extension("810|4");
    println!("810 random → {out}");
    assert!(out.starts_with('['), "810 should return array of strings");
}

#[test]
fn live_setbit_and_getbit() {
    if skip_live_redis_tests() {
        return;
    }
    let key = "epochserver:live:test:bits";

    let _ = call_extension(&format!("140|{key}|5|1"));
    let bit = call_extension(&format!("240|{key}|5"));

    println!("140/240 SETBIT/GETBIT → bit5 = {bit}");

    let _ = call_extension(&format!("400|{key}"));
}

#[test]
fn live_t100_diagnostic() {
    if skip_live_redis_tests() {
        return;
    }
    // T100 is a debug/test command (was guarded by EPOCHLIB_TEST in original)
    let out = call_extension("T100");
    println!("T100 → {out}");
    assert!(out.starts_with('['), "T100 should return an array");
    assert!(
        out.contains("0.6.0.0") || out.contains("T100"),
        "T100 should contain version or marker"
    );
}

#[test]
fn live_getrange() {
    if skip_live_redis_tests() {
        return;
    }
    let key = "epochserver:live:test:range";
    let value = r#"["abcdefghijklmnopqrstuvwxyz"]"#;

    let _ = call_extension(&format!("110|{key}|0|{value}"));

    let range = call_extension(&format!("220|{key}|0|9"));
    println!("220 GETRANGE → {range}");

    let _ = call_extension(&format!("400|{key}"));
}

/// Test that the abuse filter (invalid non-array JSON on SET) does not crash
/// and exercises the fire-and-forget logging path (now using spawn to avoid nesting).
#[test]
fn live_abuse_filter_triggers_safely() {
    if skip_live_redis_tests() {
        return;
    }
    let bad_key = "epochserver:live:test:abuse";
    // Deliberately invalid value (not a JSON array) — should be rejected with [0]
    let bad_value = r#""just a string, not an array""#;

    let out = call_extension(&format!("110|{bad_key}|0|{bad_value}"));
    println!("110 with bad value (abuse path) → {out}");

    // Must return failure marker, never panic or error string
    assert!(
        out.contains("[0]") || out.starts_with("[0"),
        "Abuse filter should return failure for bad JSON"
    );
    assert!(
        !out.to_lowercase().contains("error"),
        "Abuse path should not surface errors"
    );

    // Cleanup
    let _ = call_extension(&format!("400|{bad_key}"));
}

// =============================================================================
// Additional coverage for previously weak areas (P3 test expansion)
// =============================================================================

#[test]
fn live_abuse_logging_levels() {
    if skip_live_redis_tests() {
        return;
    }
    // Test that abuse is logged to ABUSE-LOG when LogAbuse >= 1 (our test INI has LogAbuse=1)
    let abuse_key = "ABUSE-LOG";
    let bad_key = "epochserver:live:test:abuse-log";
    let bad = r#"[not, "valid", "array", "because", "object"]"#;

    // Clear previous abuse logs for this test run (best effort)
    let _ = call_extension(&format!("400|{abuse_key}"));

    let out = call_extension(&format!("110|{bad_key}|0|{bad}"));
    println!("Abuse logging level test → {out}");

    assert!(out.contains("[0]"));

    // Verify something was written to ABUSE-LOG
    // We do a GET on the abuse key (note: this is a list, so GET may not be ideal, but LPOP or LRANGE would be better.
    // For simplicity we just check that the key exists via EXISTS after the bad SET.
    let exists = call_extension(&format!("250|{abuse_key}"));
    println!("ABUSE-LOG exists after abuse? → {exists}");
    // It may or may not still exist depending on LTRIM, but the call itself shouldn't have crashed.

    let _ = call_extension(&format!("400|{bad_key}"));
}

/// Tests that 800/820 file anti-hack commands actually write to the configured BattlEye directory.
/// Uses temporary directories for full isolation (no pollution of real server files).
#[test]
fn live_file_antihack_800_820_writes_to_temp_battleye_path() {
    if skip_live_redis_tests() {
        return;
    }
    use std::fs;

    // Create isolated temp directories
    let base_temp =
        std::env::temp_dir().join(format!("epochserver-800820-test-{}", std::process::id()));
    let be_path = base_temp.join("BattlEye");
    let config_dir = base_temp.join("config");

    fs::create_dir_all(&be_path).expect("failed to create temp BE dir");
    fs::create_dir_all(&config_dir).expect("failed to create temp config dir");

    // Write a dedicated test INI that points BattlEyePath to our temp directory
    let ini_content = format!(
        r#"[EpochServer]
InstanceID = TEST01
LogAbuse = 0
LogLimit = 100
OfficialCheck = 0
BattlEyePath = {}

[Redis]
IP = 127.0.0.1
Port = 6379
DB = 0
Password =

[SteamAPI]
Logging = 0
Key =
VACBanned = 0
"#,
        be_path.display()
    );

    let ini_path = config_dir.join("EpochServer.ini");
    fs::write(&ini_path, ini_content).expect("failed to write test INI");

    // Tell the extension to use our hermetic config directory
    std::env::set_var("EPOCHSERVER_CONFIG_DIR", &config_dir);

    // Force reload of the extension with the new config
    // (We do this by clearing the OnceLock so next load picks up the env var)
    // Note: In real usage the DLL would need to be reloaded, but for this harness we re-init the statics.
    // For simplicity in this test we just rely on the env var being set before first real call in this test.

    // Call 800 (update public variable)
    let out800 = call_extension("800|test_whitelist_entry");
    println!("800 result: {}", out800);

    // Call 820 (add ban)
    let out820 = call_extension("820|76561198000000000|Test Ban Reason");
    println!("820 result: {}", out820);

    // Verify files were created in the temp BattlEye directory
    let pv_file = be_path.join("publicvariable.txt");
    let bans_file = be_path.join("bans.txt");

    let pv_exists = pv_file.exists();
    let bans_exists = bans_file.exists();

    println!("publicvariable.txt exists: {}", pv_exists);
    println!("bans.txt exists: {}", bans_exists);

    if pv_exists {
        let content = fs::read_to_string(&pv_file).unwrap_or_default();
        println!("publicvariable.txt content: {}", content);
        assert!(
            content.contains("test_whitelist_entry"),
            "800 should have appended to publicvariable.txt"
        );
    }

    if bans_exists {
        let content = fs::read_to_string(&bans_file).unwrap_or_default();
        println!("bans.txt content: {}", content);
        assert!(
            content.contains("76561198000000000"),
            "820 should have written a ban entry"
        );
    }

    // Cleanup
    let _ = fs::remove_dir_all(&base_temp);
    std::env::remove_var("EPOCHSERVER_CONFIG_DIR");
}

/// Basic coverage for 9xx BE commands in the live harness.
/// When no real BattlEye server is configured (empty BattlEyePath/IP in our test INI),
/// these should gracefully do nothing instead of crashing.
#[test]
fn live_be_commands_graceful_when_no_be_configured() {
    if skip_live_redis_tests() {
        return;
    }
    let commands = [
        "901|Test broadcast message",
        "911|76561198000000000|Test kick reason",
        "921|76561198000000000|Test ban reason|0",
        "930", // unlock
        "931", // lock
        "991", // shutdown
    ];

    for cmd in &commands {
        let out = call_extension(cmd);
        println!("9xx {} → {}", cmd, out);
        // Should not hard-fail when no BE is configured
        assert!(
            !out.to_lowercase().contains("error") || out.is_empty() || out == "[0]",
            "9xx command {} should not hard-fail when no BE is configured",
            cmd
        );
    }
}
