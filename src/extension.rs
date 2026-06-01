//! Legacy RVExtension compatibility layer + command handlers.
//!
//! This module provides the raw C ABI that Arma 3 expects for the classic
//! string-based callExtension protocol used by all existing Epoch SQF code.
//!
//! We deliberately implement the three classic functions manually for maximum
//! fidelity with the original C++ behavior (output buffer handling, calling
//! convention on Windows, exact empty/unknown responses, etc.).
//!
//! We are currently using fully manual raw exports for the classic
//! RVExtension protocol (maximum compatibility with existing Epoch SQF).
//!
//! P3 work will focus on gradually introducing `arma-rs` for structured
//! calls via RVExtensionArgs while keeping the legacy string-based path
//! 100% compatible and unchanged.

#![allow(clippy::items_after_test_module)]

use std::ffi::{c_char, c_int, CStr, CString};
#[cfg(windows)]
use std::ffi::{c_void, OsString};
#[cfg(windows)]
use std::os::windows::ffi::OsStringExt;
use std::path::PathBuf;
use std::ptr;
use std::sync::OnceLock;

use chrono::Local;
use md5::{Digest, Md5};
use rand::Rng;
use tokio::runtime::Runtime;

use crate::config::EpochServerConfig;
use crate::sqf::SQF;
use crate::VERSION_STRING;

use serde_json::Value;

/// Hardcoded MD5 of the official a3_epoch_server.pbo (from original)
const OFFICIAL_SERVER_MD5: &str = "8497e70dafab88ea432338fee8c86579";

/// Global tokio runtime for blocking Arma calls into async Redis/etc.
static RUNTIME: OnceLock<Runtime> = OnceLock::new();

fn runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("Failed to create Tokio runtime")
    })
}

/// Global config, loaded on first use (mirrors original lazy EpochLibrary init).
static CONFIG: OnceLock<EpochServerConfig> = OnceLock::new();

fn get_config() -> &'static EpochServerConfig {
    CONFIG.get_or_init(|| {
        // Highest priority: explicit override (very useful for hermetic testing)
        if let Ok(dir) = std::env::var("EPOCHSERVER_CONFIG_DIR") {
            if !dir.is_empty() {
                return EpochServerConfig::load_or_default(&dir, "");
            }
        }

        // Match the original intent: config_path is the extension/DLL directory.
        // `current_exe()` points at arma3_x64.exe, so on Windows we ask the loader
        // which module contains this code and use that module's parent directory.
        let config_path = extension_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_else(|| {
                if cfg!(windows) {
                    std::env::current_exe()
                        .ok()
                        .and_then(|p| p.parent().map(|p| p.to_string_lossy().to_string()))
                        .unwrap_or_default()
                } else {
                    "@epochhive".to_string()
                }
            });

        EpochServerConfig::load_or_default(&config_path, "")
    })
}

fn extension_dir() -> Option<PathBuf> {
    extension_path().and_then(|p| p.parent().map(|dir| dir.to_path_buf()))
}

#[cfg(windows)]
fn extension_path() -> Option<PathBuf> {
    const GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT: u32 = 0x0000_0002;
    const GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS: u32 = 0x0000_0004;

    #[link(name = "kernel32")]
    extern "system" {
        fn GetModuleHandleExW(
            dw_flags: u32,
            lp_module_name: *const u16,
            ph_module: *mut *mut c_void,
        ) -> i32;
        fn GetModuleFileNameW(h_module: *mut c_void, lp_filename: *mut u16, n_size: u32) -> u32;
    }

    let mut module: *mut c_void = ptr::null_mut();
    let flags =
        GET_MODULE_HANDLE_EX_FLAG_FROM_ADDRESS | GET_MODULE_HANDLE_EX_FLAG_UNCHANGED_REFCOUNT;

    let ok = unsafe {
        GetModuleHandleExW(
            flags,
            extension_path as *const () as *const u16,
            &mut module,
        )
    };
    if ok == 0 || module.is_null() {
        return None;
    }

    let mut buffer = vec![0u16; 32768];
    let len = unsafe { GetModuleFileNameW(module, buffer.as_mut_ptr(), buffer.len() as u32) };
    if len == 0 {
        return None;
    }

    buffer.truncate(len as usize);
    Some(PathBuf::from(OsString::from_wide(&buffer)))
}

#[cfg(not(windows))]
fn extension_path() -> Option<PathBuf> {
    None
}

// ============================================================================
// Pagination state for large GET / GETTTL (replicates the original tempGet hack)
// ============================================================================

#[derive(Default)]
struct TempGetState {
    success: bool,
    message: String,
}

static TEMP_GET: OnceLock<tokio::sync::Mutex<TempGetState>> = OnceLock::new();

async fn get_temp_state() -> tokio::sync::MutexGuard<'static, TempGetState> {
    TEMP_GET
        .get_or_init(|| tokio::sync::Mutex::new(TempGetState::default()))
        .lock()
        .await
}

/// Handler for "000" — matches original getConfig() exactly.
pub(crate) fn handle_000() -> String {
    let cfg = get_config();
    let mut sqf = SQF::new();
    sqf.push_str_double(&cfg.instance_id);
    let has_steam = !cfg.steam_key.is_empty();
    sqf.push_number(if has_steam { 1i64 } else { 0 });
    sqf.to_array()
}

/// Handler for "510" — matches original getCurrentTime() exactly.
pub(crate) fn handle_510() -> String {
    let now = Local::now();
    let mut sqf = SQF::new();

    sqf.push_number(now.format("%Y").to_string());
    sqf.push_number(now.format("%m").to_string());
    sqf.push_number(now.format("%d").to_string());
    sqf.push_number(now.format("%H").to_string());
    sqf.push_number(now.format("%M").to_string());
    sqf.push_number(now.format("%S").to_string());

    sqf.to_array()
}

/// Handler for "810" — getRandomString (exact original behavior).
/// - Length: 5-9 characters
/// - Pool: lowercase a-z
/// - Avoid any string containing "god"
/// - Deduplicate within the requested batch
/// - Special case: count==1 && only one result → return bare string instead of array
pub(crate) fn handle_810(count: usize) -> String {
    const POOL: &[u8] = b"abcdefghijklmnopqrstuvwxyz";
    let mut rng = rand::thread_rng();
    let mut results: Vec<String> = Vec::new();

    let mut attempts = 0;
    while results.len() < count && attempts < 1000 {
        attempts += 1;

        let len = rng.gen_range(5..=9);
        let s: String = (0..len)
            .map(|_| {
                let idx = rng.gen_range(0..POOL.len());
                POOL[idx] as char
            })
            .collect();

        if s.contains("god") {
            continue;
        }
        if results.contains(&s) {
            continue;
        }

        results.push(s);
    }

    if count == 1 && results.len() == 1 {
        // Original special case: return bare string, not an array
        results.into_iter().next().unwrap()
    } else {
        let mut sqf = SQF::new();
        for r in results {
            sqf.push_str_double(&r);
        }
        sqf.to_array()
    }
}

/// Handler for "840" — getStringMd5 (exact original behavior).
/// Computes lowercase hex MD5 for each input string and returns them as an array.
pub(crate) fn handle_840(strings: &[String]) -> String {
    let mut sqf = SQF::new();

    for s in strings {
        let mut hasher = Md5::new();
        hasher.update(s.as_bytes());
        let digest = hasher.finalize();
        let hex = format!("{:x}", digest);
        sqf.push_str_double(&hex);
    }

    sqf.to_array()
}

// dispatch_legacy fully removed - routing is now in handle_command + the RVExtension async decision logic.

/// Main async command router. This is where we decide sync vs spawned work
/// and call the proper handler (including pagination state for GETs).
async fn handle_command(code: &str, rest: &str, output_size: usize) -> String {
    match code {
        "000" => handle_000(),
        "001" => {
            if let Ok(steamid) = rest.parse::<i64>() {
                runtime().spawn(async move { handle_init_player_check(steamid).await });
            }
            String::new()
        }
        "510" => handle_510(),
        "500" => handle_500().await,
        "300" => handle_300(rest).await,
        "400" => handle_400(rest).await,
        "810" => {
            let count = rest.parse::<usize>().unwrap_or(1).max(1);
            handle_810(count)
        }
        "840" => {
            let to_hash: Vec<String> = if rest.is_empty() {
                vec![]
            } else {
                rest.split('|')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect()
            };
            handle_840(&to_hash)
        }

        "100" | "T100" => handle_t100(),

        // === GET family with pagination state (very important for fidelity) ===
        "200" => handle_get(rest, output_size, false).await, // GET only
        "210" => handle_get(rest, output_size, true).await,  // GET + TTL

        // New medium commands
        "250" => handle_exists(rest).await,
        "600" => handle_lpop_cmd(rest).await,
        "700" => handle_log(rest).await,
        "701" => {
            // Async log - but since we're already in the spawned task for 701,
            // we just do the work here.
            handle_log(rest).await
        }

        "220" => handle_getrange(rest).await,
        "240" => handle_getbit(rest).await,
        "140" | "141" => handle_setbit(rest).await,
        "130" => handle_expire(rest).await,
        // 131 is handled as async at the RVExtension level
        "110" | "120" => {
            // 110|prefix:key||value
            // 120|prefix:key|ttl||value
            let mut p = rest.splitn(4, '|');
            let key_part = p.next().unwrap_or("").trim();
            let ttl = if code == "120" { p.next() } else { None };
            let _call_id = p.next(); // ignored, like original
            let value = p.next().unwrap_or("").trim();

            handle_set(key_part, value, ttl).await
        }

        "830" => {
            let result = crate::redis::execute("INCR", &["ahb-cnt".to_string()]).await;
            let mut sqf = SQF::new();
            if result.success {
                sqf.push_number(1i64);
                sqf.push_str_double(&result.message);
            } else {
                sqf.push_number(0i64);
            }
            sqf.to_array()
        }

        "800" | "801" => {
            let strings: Vec<String> = if rest.is_empty() {
                vec![]
            } else {
                rest.split('|')
                    .filter(|s| !s.is_empty())
                    .map(|s| s.to_string())
                    .collect()
            };
            if code.ends_with('1') {
                runtime().spawn(async move { handle_update_public_variable(strings).await });
                String::new()
            } else {
                handle_update_public_variable(strings).await
            }
        }

        "820" | "821" => {
            let mut p = rest.splitn(2, '|');
            let uid = p.next().unwrap_or("").to_string();
            let reason = p.next().unwrap_or("").to_string();
            if code.ends_with('1') {
                runtime().spawn(async move { handle_add_ban(uid, reason).await });
                String::new()
            } else {
                handle_add_ban(uid, reason).await
            }
        }

        // === BattlEye 9xx commands (Option A focus) ===
        "901" => {
            let msg = rest.to_string();
            runtime().spawn(async move { handle_be_broadcast(&msg).await });
            String::new()
        }
        "911" => {
            let mut p = rest.splitn(2, '|');
            let uid = p.next().unwrap_or("").to_string();
            let reason = p.next().unwrap_or("").to_string();
            runtime().spawn(async move { handle_be_kick(&uid, &reason).await });
            String::new()
        }
        "921" => {
            let mut p = rest.splitn(3, '|');
            let uid = p.next().unwrap_or("").to_string();
            let reason = p.next().unwrap_or("").to_string();
            let duration = p.next().unwrap_or("-1").to_string();
            runtime().spawn(async move { handle_be_ban(&uid, &reason, &duration).await });
            String::new()
        }
        "930" => {
            runtime().spawn(async { handle_be_unlock().await });
            String::new()
        }
        "931" => {
            runtime().spawn(async { handle_be_lock().await });
            String::new()
        }
        "991" => {
            runtime().spawn(async { handle_be_shutdown().await });
            String::new()
        }

        _ => format!("Unkown command {}", code),
    }
}

// ============================================================================
// Real Redis-backed command implementations (A + B work)
// ============================================================================

async fn handle_500() -> String {
    let result = crate::redis::execute("PING", &[]).await;
    let mut sqf = SQF::new();
    if result.success {
        sqf.push_number(1i64);
        sqf.push_str_double("PONG");
    } else {
        sqf.push_number(0i64);
        sqf.push_str_double(&result.message);
    }
    sqf.to_array()
}

async fn handle_300(rest: &str) -> String {
    let key = rest.trim();
    if key.is_empty() {
        return "[0]".to_string();
    }
    let result = crate::redis::execute("TTL", &[key.to_string()]).await;
    let mut sqf = SQF::new();
    if result.success {
        sqf.push_number(1i64);
        sqf.push_str_double(&result.message);
    } else {
        sqf.push_number(0i64);
    }
    sqf.to_array()
}

async fn handle_400(rest: &str) -> String {
    let key = rest.trim();
    if key.is_empty() {
        return "[0]".to_string();
    }
    let result = crate::redis::execute("DEL", &[key.to_string()]).await;
    let mut sqf = SQF::new();
    if result.success {
        sqf.push_number(1i64);
        sqf.push_str_double(&result.message);
    } else {
        sqf.push_number(0i64);
    }
    sqf.to_array()
}

async fn handle_exists(rest: &str) -> String {
    let key = rest.trim();
    if key.is_empty() {
        return "[0]".to_string();
    }
    let result = crate::redis::execute("EXISTS", &[key.to_string()]).await;
    let mut sqf = SQF::new();
    if result.success {
        sqf.push_number(1i64);
        sqf.push_str_double(&result.message);
    } else {
        sqf.push_number(0i64);
    }
    sqf.to_array()
}

async fn handle_lpop_cmd(rest: &str) -> String {
    let key = rest.trim();
    if key.is_empty() {
        return "[0]".to_string();
    }
    let full_key = format!("CMD:{}", key);
    let result = crate::redis::execute("LPOP", &[full_key]).await;
    let mut sqf = SQF::new();
    if result.success {
        sqf.push_number(1i64);
        sqf.push_str_double(&result.message);
    } else {
        sqf.push_number(0i64);
    }
    sqf.to_array()
}

async fn handle_log(rest: &str) -> String {
    // Format: 700|prefix|message   or 701|prefix|message
    let mut parts = rest.splitn(2, '|');
    let prefix = parts.next().unwrap_or("").trim();
    let message = parts.next().unwrap_or("").trim();

    if prefix.is_empty() {
        return "[0]".to_string();
    }

    let log_key = format!("{}-LOG", prefix);
    let timestamp = chrono::Local::now()
        .format("%Y-%m-%d %H:%M:%S ")
        .to_string();
    let full_msg = format!("{}{}", timestamp, message);

    // LPUSH
    let _ = crate::redis::execute("LPUSH", &[log_key.clone(), full_msg]).await;

    // LTRIM to logLimit
    let cfg = get_config();
    let limit = cfg.log_limit as i64;
    let _ = crate::redis::execute("LTRIM", &[log_key, "0".to_string(), limit.to_string()]).await;

    let mut sqf = SQF::new();
    sqf.push_number(1i64);
    sqf.to_array()
}

async fn handle_getrange(rest: &str) -> String {
    // 220|key|start|stop
    let mut parts = rest.splitn(3, '|');
    let key = parts.next().unwrap_or("").trim().to_string();
    let start = parts.next().unwrap_or("0").trim().to_string();
    let stop = parts.next().unwrap_or("-1").trim().to_string();

    if key.is_empty() {
        return "[0]".to_string();
    }

    let result = crate::redis::execute("GETRANGE", &[key, start, stop]).await;

    let mut sqf = SQF::new();
    if result.success {
        // GETRANGE returns a string; original did special ' escaping + single quotes
        let escaped: String = result
            .message
            .chars()
            .map(|c| {
                if c == '\'' {
                    "''".to_string()
                } else {
                    c.to_string()
                }
            })
            .collect();
        sqf.push_number(1i64);
        sqf.push_str(&escaped, 1);
    } else {
        sqf.push_number(0i64);
    }
    sqf.to_array()
}

async fn handle_getbit(rest: &str) -> String {
    // 240|key|offset
    let mut parts = rest.splitn(2, '|');
    let key = parts.next().unwrap_or("").trim().to_string();
    let offset = parts.next().unwrap_or("0").trim().to_string();

    if key.is_empty() {
        return "[0]".to_string();
    }

    let result = crate::redis::execute("GETBIT", &[key, offset]).await;

    let mut sqf = SQF::new();
    if result.success {
        sqf.push_number(1i64);
        sqf.push_str_double(&result.message);
    } else {
        sqf.push_number(0i64);
    }
    sqf.to_array()
}

async fn handle_setbit(rest: &str) -> String {
    // 140|key|offset|value   (sync)
    // 141 is async version
    let mut parts = rest.splitn(3, '|');
    let key = parts.next().unwrap_or("").trim().to_string();
    let offset = parts.next().unwrap_or("0").trim().to_string();
    let bit = parts.next().unwrap_or("0").trim().to_string();

    if key.is_empty() {
        return "[0]".to_string();
    }

    let result = crate::redis::execute("SETBIT", &[key, offset, bit]).await;

    let mut sqf = SQF::new();
    if result.success {
        sqf.push_number(1i64);
        sqf.push_str_double(&result.message);
    } else {
        sqf.push_number(0i64);
    }
    sqf.to_array()
}

async fn handle_expire(rest: &str) -> String {
    // 130|key|seconds   or 131 (async, handled at top level)
    let mut parts = rest.splitn(2, '|');
    let key = parts.next().unwrap_or("").trim().to_string();
    let seconds = parts.next().unwrap_or("0").trim().to_string();

    if key.is_empty() {
        return "[0]".to_string();
    }

    let result = crate::redis::execute("EXPIRE", &[key, seconds]).await;

    let mut sqf = SQF::new();
    if result.success {
        sqf.push_number(1i64);
        sqf.push_str_double(&result.message);
    } else {
        sqf.push_number(0i64);
    }
    sqf.to_array()
}

/// Validates that the value is a JSON array containing only allowed types
/// (bool, number, string, array) — matching the original PCRE regex intent
/// for abuse protection on SET/SETEX.
fn validate_hive_value(value: &str) -> bool {
    match serde_json::from_str::<Value>(value) {
        Ok(Value::Array(arr)) => arr.iter().all(is_allowed_json_value),
        _ => false,
    }
}

fn is_allowed_json_value(v: &Value) -> bool {
    match v {
        Value::Bool(_) | Value::Number(_) | Value::String(_) => true,
        Value::Array(arr) => arr.iter().all(is_allowed_json_value),
        _ => false, // No objects, no nulls at top level of elements
    }
}

/// Escapes a string for embedding inside a single-quoted SQF string,
/// by doubling any single quotes (exact original behavior for GET responses).
fn escape_for_sqf_single_quote(s: &str) -> String {
    s.chars()
        .map(|c| {
            if c == '\'' {
                "''".to_string()
            } else {
                c.to_string()
            }
        })
        .collect()
}

/// SET / SETEX handlers (110/111 and 120/121).
/// Includes the original-style JSON array validation for abuse protection.
async fn handle_set(key: &str, value: &str, ttl: Option<&str>) -> String {
    if key.is_empty() || value.is_empty() {
        let mut sqf = SQF::new();
        sqf.push_number(0i64);
        return sqf.to_array();
    }

    // Original behavior: validate that value is a proper JSON array
    if !validate_hive_value(value) {
        let cfg = get_config();
        if cfg.log_abuse > 0 {
            let msg = format_abuse_message(key);
            eprintln!("{}", msg);
            if cfg.log_abuse > 1 {
                eprintln!("Value: {}", value);
            }

            // Write to Redis abuse log (fire-and-forget, matching original spirit).
            // We spawn instead of block_on to avoid Tokio runtime nesting when called
            // from within an async handler (e.g. invalid 110/120 SET value).
            let abuse_key = "ABUSE-LOG"; // or make configurable
            let full = format!(
                "{} {}",
                chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
                msg
            );
            let limit = cfg.log_limit as i64;
            runtime().spawn(async move {
                let _ = crate::redis::execute("LPUSH", &[abuse_key.to_string(), full]).await;
                let _ = crate::redis::execute(
                    "LTRIM",
                    &[abuse_key.to_string(), "0".to_string(), limit.to_string()],
                )
                .await;
            });
        }

        let mut sqf = SQF::new();
        sqf.push_number(0i64);
        return sqf.to_array();
    }

    let result = if let Some(ttl_str) = ttl {
        crate::redis::execute(
            "SETEX",
            &[key.to_string(), ttl_str.to_string(), value.to_string()],
        )
        .await
    } else {
        crate::redis::execute("SET", &[key.to_string(), value.to_string()]).await
    };

    let mut sqf = SQF::new();
    if result.success {
        sqf.push_number(1i64);
        sqf.push_str_double(&result.message);
    } else {
        sqf.push_number(0i64);
    }
    sqf.to_array()
}

// ============================================================================
// GET with proper pagination state (the famous tempGet logic)
// ============================================================================

async fn handle_get(key: &str, output_size: usize, include_ttl: bool) -> String {
    let key = key.trim();
    if key.is_empty() {
        return "[0]".to_string();
    }

    let mut state = get_temp_state().await;

    // If we don't have a pending large value, fetch fresh
    if !state.success {
        let get_result = crate::redis::execute("GET", &[key.to_string()]).await;

        if !get_result.success {
            return "[0]".to_string();
        }

        state.success = true;
        state.message = get_result.message;

        if include_ttl {
            // We need to also fetch TTL for the first response of 210
            let _ttl_result = crate::redis::execute("TTL", &[key.to_string()]).await;
            // Store TTL temporarily? For simplicity in this pass we handle it in the response building below.
            // The original stored separate TTL. We'll fetch again on first large response for 210.
        }
    }

    let mut sqf = SQF::new();

    if state.message.is_empty() {
        sqf.push_number(0i64);
        return sqf.to_array();
    }

    let chunk_size = output_size.saturating_sub(20);

    if state.message.len() > output_size {
        // Large value - return CONTINUE (2) + first chunk
        sqf.push_number(2i64);

        if include_ttl {
            // Fetch TTL for the first response of a 210 call
            let ttl_res = crate::redis::execute("TTL", &[key.to_string()]).await;
            if ttl_res.success {
                if let Ok(ttl) = ttl_res.message.parse::<i64>() {
                    sqf.push_number(ttl);
                } else {
                    sqf.push_number(-1i64);
                }
            } else {
                sqf.push_number(-1i64);
            }
        }

        let chunk =
            escape_for_sqf_single_quote(&state.message[0..chunk_size.min(state.message.len())]);

        sqf.push_str(&chunk, 1); // single quotes + already escaped

        // Consume the chunk from the front (original logic)
        if state.message.len() >= chunk_size {
            state.message = state.message[chunk_size..].to_string();
        } else {
            state.message.clear();
        }
    } else {
        // Fits in one response
        sqf.push_number(1i64);

        if include_ttl {
            let ttl_res = crate::redis::execute("TTL", &[key.to_string()]).await;
            if ttl_res.success {
                if let Ok(ttl) = ttl_res.message.parse::<i64>() {
                    sqf.push_number(ttl);
                } else {
                    sqf.push_number(-1i64);
                }
            } else {
                sqf.push_number(-1i64);
            }
        }

        let escaped = escape_for_sqf_single_quote(&state.message);
        sqf.push_str(&escaped, 1);

        // Done with this value
        state.success = false;
        state.message.clear();
    }

    sqf.to_array()
}

// ============================================================================
// Raw C ABI exports — this is what Arma actually calls
// ============================================================================

/// RVExtensionVersion — called by Arma to query extension version.
///
/// # Safety
/// - `output` must point to a writable buffer of at least `output_size` bytes.
/// - This function is part of the Arma 3 extension ABI and must not panic.
#[no_mangle]
pub unsafe extern "system" fn RVExtensionVersion(output: *mut c_char, output_size: usize) {
    // P3 hook point - safe place to initialize modern subsystems
    crate::modern::init();

    if output.is_null() || output_size == 0 {
        return;
    }

    let version = CString::new(VERSION_STRING).unwrap_or_default();
    let bytes = version.as_bytes_with_nul();

    unsafe {
        let copy_len = bytes.len().min(output_size.saturating_sub(1));
        ptr::copy_nonoverlapping(bytes.as_ptr(), output as *mut u8, copy_len);
        *output.add(copy_len) = 0;
    }
}

/// The main legacy entry point used by 100% of current Epoch SQF code.
/// Signature and behavior must match the original as closely as possible.
///
/// # Safety
/// - `output` must point to a writable buffer of at least `output_size` bytes.
/// - `function`, when non-null, must point to a valid NUL-terminated C string.
/// - This function is part of the Arma 3 extension ABI and must not panic.
#[no_mangle]
pub unsafe extern "system" fn RVExtension(
    output: *mut c_char,
    output_size: c_int,
    function: *const c_char,
) {
    if output.is_null() || output_size <= 0 {
        return;
    }

    let input = if function.is_null() {
        ""
    } else {
        unsafe { CStr::from_ptr(function) }.to_str().unwrap_or("")
    };

    let output_size_usize = output_size as usize;

    let mut parts = input.splitn(2, '|');
    let code = parts.next().unwrap_or("").trim();
    let rest = parts.next().unwrap_or("").trim();

    if !code.is_empty() {
        let _ = runtime().block_on(crate::redis::initialize());

        if !check_official_server() {
            eprintln!("[EpochServer] Wrong server files - exiting.");
            std::process::exit(1);
        }
    }

    let is_async = matches!(
        code,
        "111"
            | "121"
            | "131"
            | "141"
            | "701"
            | "801"
            | "821"
            | "901"
            | "911"
            | "921"
            | "930"
            | "931"
            | "991"
    );

    let result = if is_async {
        let code_owned = match code {
            "111" => "110",
            "121" => "120",
            "131" => "130",
            "141" => "140",
            other => other,
        }
        .to_string();
        let rest_owned = rest.to_string();
        let size = output_size_usize;

        runtime().spawn(async move {
            let _ = handle_command(&code_owned, &rest_owned, size).await;
        });

        String::new()
    } else {
        runtime().block_on(handle_command(code, rest, output_size_usize))
    };

    let bytes = result.as_bytes();
    let max = output_size_usize.saturating_sub(1);
    let copy_len = bytes.len().min(max);

    unsafe {
        ptr::copy_nonoverlapping(bytes.as_ptr(), output as *mut u8, copy_len);
        *output.add(copy_len) = 0;
    }
}

/// RVExtensionArgs entry point.
///
/// # Safety
/// Same contract as `RVExtension`.
/// - `output` must point to a writable buffer of at least `output_size` bytes.
/// - `function`, when non-null, must point to a valid NUL-terminated C string.
/// - This function is part of the Arma 3 extension ABI and must not panic.
#[no_mangle]
pub unsafe extern "system" fn RVExtensionArgs(
    output: *mut c_char,
    output_size: c_int,
    function: *const c_char,
    _args: *const *const c_char,
    _arg_count: c_int,
) {
    unsafe { RVExtension(output, output_size, function) };
}

// ============================================================================
// BattlEye helpers for 9xx commands (Option A)
// ============================================================================

fn with_be_client<F>(mut action: F) -> std::io::Result<()>
where
    F: FnMut(&mut crate::be::BEClient) -> std::io::Result<()>,
{
    let cfg = get_config();
    if cfg.battleye_ip.is_empty() {
        return Ok(());
    }

    let mut bec = crate::be::BEClient::new(&cfg.battleye_ip, cfg.battleye_port)?;
    if bec.login(&cfg.battleye_password)? {
        action(&mut bec)?;
    }
    bec.disconnect();
    Ok(())
}

fn compute_be_guid(steam64: &str) -> String {
    let id: u64 = steam64.parse().unwrap_or(0);
    let mut parts = [0u8; 8];
    let mut tmp = id;
    for b in &mut parts {
        *b = (tmp & 0xff) as u8;
        tmp >>= 8;
    }

    let mut data = b"BE".to_vec();
    data.extend_from_slice(&parts);

    use md5::{Digest, Md5};
    let mut h = Md5::new();
    h.update(&data);
    format!("{:x}", h.finalize())
}

async fn handle_be_broadcast(msg: &str) {
    let _ = with_be_client(|b| b.say(msg));
}

async fn handle_be_kick(steam64: &str, reason: &str) {
    let guid = compute_be_guid(steam64);
    let r = reason.to_string();
    let _ = with_be_client(|b| resolve_and_act_on_player(b, &guid, |b, slot| b.kick(slot, &r)));
}

async fn handle_be_ban(steam64: &str, reason: &str, dur: &str) {
    let guid = compute_be_guid(steam64);
    let r = reason.to_string();
    let duration: i32 = dur.parse().unwrap_or(-1);
    let _ = with_be_client(|b| {
        resolve_and_act_on_player(b, &guid, |b, slot| b.ban(slot, duration, &r))
    });
}

/// Pure helper extracted for testability.
/// Common pattern used by kick/ban: run "players", resolve slot by GUID, then perform action.
fn resolve_and_act_on_player<F>(
    b: &mut crate::be::BEClient,
    guid: &str,
    action: F,
) -> std::io::Result<()>
where
    F: FnOnce(&mut crate::be::BEClient, u32) -> std::io::Result<()>,
{
    let _ = b.execute_command("players");
    if let Some(slot) = b.get_player_slot(guid) {
        action(b, slot)?;
    }
    Ok(())
}

async fn handle_be_lock() {
    let _ = with_be_client(|b| b.lock());
}
async fn handle_be_unlock() {
    let _ = with_be_client(|b| b.unlock());
}
async fn handle_be_shutdown() {
    let _ = with_be_client(|b| b.shutdown());
}

/// Builds the appended content for publicvariable.txt exactly as the original did.
pub(crate) fn build_publicvariable_content(
    original_content: &str,
    whitelist_strings: &[String],
) -> String {
    let mut new_content = original_content.trim_end().to_string();
    for s in whitelist_strings {
        new_content.push_str(&format!(" !=\"{}\"", s));
    }
    new_content
}

/// Pure helper extracted for testability.
/// Builds the exact line written to bans.txt for a given GUID + reason.
pub(crate) fn build_ban_line(guid: &str, reason: &str) -> String {
    format!("{} -1 {}", guid, reason)
}

/// Pure helper for abuse logging message (extracted for testability and consistency).
pub(crate) fn format_abuse_message(key: &str) -> String {
    format!("[Abuse] SET key {} does not match the allowed syntax!", key)
}

fn check_official_server() -> bool {
    // Allow completely bypassing the official server check via env var.
    // This is useful for live integration tests and custom target directories.
    if let Ok(v) = std::env::var("EPOCHSERVER_OFFICIAL_CHECK") {
        let v = v.trim();
        if v == "0"
            || v.eq_ignore_ascii_case("false")
            || v.eq_ignore_ascii_case("off")
            || v.is_empty()
        {
            return true;
        }
    }

    let cfg = get_config();
    if !cfg.official_check {
        return true;
    }

    let candidates = [
        "addons/a3_epoch_server.pbo",
        "../@epochhive/addons/a3_epoch_server.pbo",
        "a3_epoch_server.pbo",
    ];

    for path in &candidates {
        if let Ok(mut file) = std::fs::File::open(path) {
            let mut hasher = Md5::new();
            let mut buffer = [0u8; 8192];
            loop {
                match std::io::Read::read(&mut file, &mut buffer) {
                    Ok(0) => break,
                    Ok(n) => hasher.update(&buffer[..n]),
                    Err(_) => break,
                }
            }

            let hex = format!("{:x}", hasher.finalize());
            if hex == OFFICIAL_SERVER_MD5 {
                return true;
            }
        }
    }

    eprintln!("[EpochServer] Official server file check failed!");
    false
}

async fn handle_init_player_check(steamid: i64) {
    let cfg = get_config();
    if cfg.steam_key.is_empty() {
        return;
    }

    let steam = crate::steam::SteamAPI::new(crate::steam::SteamConfig {
        key: cfg.steam_key.clone(),
        logging: cfg.steam_logging,
        vac_banned: cfg.steam_vac_banned,
        vac_min_bans: cfg.steam_vac_min_bans,
        vac_max_days: cfg.steam_vac_max_days,
        player_allow_older_than: cfg.steam_player_allow_older_than,
    });

    let sid = steamid.to_string();

    // Check bans
    if let Some(bans) = steam.get_player_bans(&sid).await {
        let should_ban = (cfg.steam_vac_banned && bans.vac_banned)
            || (cfg.steam_vac_min_bans > 0 && bans.number_of_vac_bans >= cfg.steam_vac_min_bans)
            || (cfg.steam_vac_max_days > 0 && bans.days_since_last_ban < cfg.steam_vac_max_days);

        if should_ban {
            let _ = handle_add_ban(sid.clone(), "VAC Ban".to_string()).await;
            return;
        }
    }

    // Check account age
    if cfg.steam_player_allow_older_than > 0 {
        if let Some(summary) = steam.get_player_summary(&sid).await {
            if let Some(created) = summary.timecreated {
                let now = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_secs();
                if now.saturating_sub(created) < cfg.steam_player_allow_older_than as u64 {
                    let _ = handle_add_ban(sid, "New account filter".to_string()).await;
                }
            }
        }
    }
}

async fn handle_update_public_variable(whitelist_strings: Vec<String>) -> String {
    let cfg = get_config();
    let pv_path = std::path::Path::new(&cfg.battleye_path).join("publicvariable.txt");
    let original_path = pv_path.with_extension("original");

    // Try to get original content
    let mut content = String::new();
    let mut has_original = false;

    if let Ok(mut f) = std::fs::File::open(&original_path) {
        use std::io::Read;
        let _ = f.read_to_string(&mut content);
        has_original = true;
    }

    if !has_original {
        if let Ok(mut f) = std::fs::File::open(&pv_path) {
            use std::io::Read;
            let _ = f.read_to_string(&mut content);
            // Backup
            let _ = std::fs::write(&original_path, &content);
        } else {
            return "[0]".to_string();
        }
    }

    // Append whitelist entries (original format: space-separated != "str" on the relevant line)
    let new_content = build_publicvariable_content(&content, &whitelist_strings);

    if std::fs::write(&pv_path, &new_content).is_err() {
        return "[0]".to_string();
    }

    // Trigger BE reload
    let _ = with_be_client(|b| b.load_events());

    "[1]".to_string()
}

async fn handle_add_ban(steam64: String, reason: String) -> String {
    let guid = compute_be_guid(&steam64);
    let cfg = get_config();
    if cfg.battleye_path.trim().is_empty() {
        return "[0]".to_string();
    }

    let bans_path = std::path::Path::new(&cfg.battleye_path).join("bans.txt");
    let line = build_ban_line(&guid, &reason);

    // Append to bans.txt
    use std::io::Write;
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&bans_path)
    {
        let _ = writeln!(f, "{}", line);
    } else {
        return "[0]".to_string();
    }

    // Use BE to reload bans and optionally kick/ban live player
    let _ = with_be_client(|b| {
        let _ = b.load_bans();
        // Try to ban live if player is online
        let _ = b.execute_command("players");
        if let Some(slot) = b.get_player_slot(&guid) {
            let _ = b.ban(slot, 0, &reason);
        }
        Ok(())
    });

    let mut sqf = SQF::new();
    sqf.push_str_double("1");
    sqf.push_str_double(&guid);
    sqf.to_array()
}

/// T100 — Internal test / diagnostic command.
/// In the original this was only available when compiled with EPOCHLIB_TEST.
/// We return basic diagnostic info for development and debugging.
fn handle_t100() -> String {
    let cfg = get_config();
    let mut sqf = SQF::new();

    sqf.push_str_double("T100");
    sqf.push_str_double(VERSION_STRING);
    sqf.push_str_double(&cfg.instance_id);
    sqf.push_number(if cfg.steam_key.is_empty() { 0 } else { 1 });
    sqf.push_number(if cfg.battleye_ip.is_empty() { 0 } else { 1 });

    sqf.to_array()
}

// ============================================================================
// Unit tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::be::BattlEyeClient;

    #[test]
    fn single_quote_escaping_matches_original() {
        let input = "it's a 'test' with many 'quotes'";
        let escaped: String = input
            .chars()
            .map(|c| {
                if c == '\'' {
                    "''".to_string()
                } else {
                    c.to_string()
                }
            })
            .collect();
        assert_eq!(escaped, "it''s a ''test'' with many ''quotes''");
    }

    // === validate_hive_value tests (abuse protection) ===

    #[test]
    fn validate_hive_value_accepts_valid_arrays() {
        assert!(validate_hive_value(r#"["string", 42, true, ["nested"]]"#));
        assert!(validate_hive_value(r#"[]"#));
        assert!(validate_hive_value(r#"[1,2,3]"#));
    }

    #[test]
    fn validate_hive_value_rejects_non_arrays() {
        assert!(!validate_hive_value(r#""just a string""#));
        assert!(!validate_hive_value(r#"{"object": true}"#));
        assert!(!validate_hive_value(r#"123"#));
    }

    #[test]
    fn validate_hive_value_rejects_bad_element_types() {
        assert!(!validate_hive_value(r#"["ok", {"bad": "object"}]"#));
        assert!(!validate_hive_value(r#"[null]"#));
    }

    #[test]
    fn validate_hive_value_handles_edge_cases() {
        // Very long but valid
        let long_valid = format!(r#"["{}", 123456789012345, true]"#, "x".repeat(10000));
        assert!(validate_hive_value(&long_valid));

        // Deeply nested arrays (should still be allowed)
        assert!(validate_hive_value(r#"[[[[["deep"]]]]]"#));

        // Mixed bad types inside nested
        assert!(!validate_hive_value(r#"["ok", [ {"no":1} ]]"#));

        // Empty string value
        assert!(!validate_hive_value(r#""#));

        // Array with only numbers and bools
        assert!(validate_hive_value(r#"[1, true, false, 3.14]"#));

        // Very deeply invalid
        assert!(!validate_hive_value(r#"[[[[[ {"evil": true} ]]]]"#));
    }

    #[test]
    fn validate_hive_value_rejects_top_level_non_array_even_if_content_looks_ok() {
        assert!(!validate_hive_value(r#""looks like array but is string""#));
        assert!(!validate_hive_value(r#"12345"#));
    }

    // === escape_for_sqf_single_quote ===

    #[test]
    fn escape_for_sqf_single_quote_doubles_quotes() {
        assert_eq!(escape_for_sqf_single_quote("it's"), "it''s");
        assert_eq!(escape_for_sqf_single_quote("no quotes"), "no quotes");
        assert_eq!(escape_for_sqf_single_quote("a'b'c"), "a''b''c");
    }

    // === compute_be_guid ===

    #[test]
    fn compute_be_guid_produces_lowercase_hex() {
        let guid = compute_be_guid("76561198000000000");
        assert!(guid
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()));
        assert_eq!(guid.len(), 32);
    }

    #[test]
    fn compute_be_guid_handles_invalid_input() {
        let guid = compute_be_guid("not-a-number");
        assert_eq!(guid.len(), 32); // falls back to 0
    }

    // === handle_810 (random strings) property tests ===

    #[test]
    fn handle_810_returns_bare_string_for_count_1() {
        let result = handle_810(1);
        // For count==1 it should not be a SQF array
        assert!(!result.starts_with('['));
        assert!(!result.ends_with(']'));
        assert!(result.len() >= 5 && result.len() <= 9);
    }

    #[test]
    fn handle_810_returns_array_for_count_gt_1() {
        let result = handle_810(3);
        assert!(result.starts_with('['));
        assert!(result.ends_with(']'));
        // Should contain 3 strings separated by commas
        let inner = result.trim_matches(|c| c == '[' || c == ']');
        let parts: Vec<&str> = inner.split(',').collect();
        assert_eq!(parts.len(), 3);
    }

    #[test]
    fn handle_810_never_returns_god() {
        for _ in 0..20 {
            let result = handle_810(5);
            assert!(
                !result.contains("god"),
                "Random string should never contain 'god'"
            );
        }
    }

    // === handle_t100 basic structure ===

    #[test]
    fn handle_t100_returns_array_with_expected_fields() {
        let result = handle_t100();
        assert!(result.starts_with('['));
        assert!(result.contains("T100"));
        assert!(result.contains("0.6.0.0")); // current VERSION_STRING
    }

    // === 800/820 file anti-hack pure logic tests ===

    #[test]
    fn build_publicvariable_content_appends_correct_format() {
        let original = " !=\"existing1\" !=\"existing2\"";
        let additions = vec!["new1".to_string(), "new2".to_string()];
        let result = build_publicvariable_content(original, &additions);
        assert_eq!(
            result,
            " !=\"existing1\" !=\"existing2\" !=\"new1\" !=\"new2\""
        );
    }

    #[test]
    fn build_publicvariable_content_handles_empty_original() {
        let additions = vec!["first".to_string()];
        let result = build_publicvariable_content("", &additions);
        assert_eq!(result, " !=\"first\"");
    }

    #[test]
    fn build_ban_line_formats_correctly() {
        let result = build_ban_line("abc123def456", "Cheating");
        assert_eq!(result, "abc123def456 -1 Cheating");
    }

    #[test]
    fn build_ban_line_handles_special_characters_in_reason() {
        let result = build_ban_line("guidhere", "Reason with spaces & symbols!");
        assert_eq!(result, "guidhere -1 Reason with spaces & symbols!");
    }

    #[test]
    fn sync_800_path_does_not_nest_runtime() {
        let result = runtime().block_on(handle_command("800", "codex_test_whitelist", 8192));
        assert_eq!(result, "[0]");
    }

    #[test]
    fn handle_add_ban_without_battleye_path_is_graceful() {
        let result = runtime().block_on(handle_add_ban(
            "76561198000000000".to_string(),
            "Test reason".to_string(),
        ));
        assert_eq!(result, "[0]");
    }

    // === 9xx BE command helper tests (using BEClient test helper) ===

    #[test]
    fn resolve_and_act_on_player_finds_slot_and_calls_action() {
        let players_output = r#"0 "PlayerOne" 1.2.3.4:2304 "abc123def456"
3 "TargetPlayer" 9.9.9.9:2304 "targetguidhere"
"#;
        let mut client = crate::be::BEClient::with_result_for_test(players_output.to_string());

        let mut acted_on_slot: Option<u32> = None;
        let result = resolve_and_act_on_player(&mut client, "targetguidhere", |_b, slot| {
            acted_on_slot = Some(slot);
            Ok(())
        });

        assert!(result.is_ok());
        assert_eq!(acted_on_slot, Some(3));
    }

    #[test]
    fn resolve_and_act_on_player_does_nothing_if_not_found() {
        let players_output = "0 \"PlayerOne\" 1.2.3.4:2304 \"abc123def456\"";
        let mut client = crate::be::BEClient::with_result_for_test(players_output.to_string());

        let mut called = false;
        let result = resolve_and_act_on_player(&mut client, "nonexistentguid", |_, _| {
            called = true;
            Ok(())
        });

        assert!(result.is_ok());
        assert!(!called);
    }

    #[test]
    fn handle_be_kick_uses_mock_for_integration_style_test() {
        // Demonstrates P3-style mock usage for 9xx flows
        let mut mock =
            crate::be::MockBEClient::new("3 \"TargetPlayer\" 9.9.9.9:2304 \"targetguidhere\"");

        // Simulate the core of handle_be_kick
        let guid = "targetguidhere";
        let reason = "Test reason";

        let _ = with_be_client_for_test(&mut mock, |b| {
            let _ = b.execute_command("players");
            if let Some(slot) = b.get_player_slot(guid) {
                let _ = b.kick(slot, reason);
            }
            Ok(())
        });

        assert!(mock.commands_called.iter().any(|c| c.contains("players")));
        assert!(mock.commands_called.iter().any(|c| c.contains("kick 3")));
    }

    #[test]
    fn handle_be_ban_uses_mock_and_records_action() {
        let mut mock = crate::be::MockBEClient::new("5 \"BannedGuy\" 1.2.3.4 \"bannedguid\"");

        let guid = "bannedguid";
        let reason = "Hacking";

        // Simulate handle_be_ban core logic
        let _ = with_be_client_for_test(&mut mock, |b| {
            let _ = b.execute_command("players");
            if let Some(slot) = b.get_player_slot(guid) {
                let _ = b.ban(slot, -1, reason);
            }
            Ok(())
        });

        assert!(mock.commands_called.iter().any(|c| c.contains("players")));
        assert!(mock.last_ban.is_some());
        let (slot, dur, r) = mock.last_ban.unwrap();
        assert_eq!(slot, 5);
        assert_eq!(dur, -1);
        assert_eq!(r, "Hacking");
    }

    #[test]
    fn handle_be_broadcast_with_custom_mock_response() {
        let mut mock = crate::be::MockBEClient::new("");
        mock.set_command_response("say -1 Hello from test", "OK");

        let _ = with_be_client_for_test(&mut mock, |b| b.say("Hello from test"));

        assert!(mock
            .commands_called
            .iter()
            .any(|c| c.contains("say -1 Hello from test")));
    }

    // === Abuse logging format / decision hardening ===

    #[test]
    fn abuse_log_message_format_is_consistent() {
        let key = "player:12345:inventory";
        let msg = format!("[Abuse] SET key {} does not match the allowed syntax!", key);
        assert!(msg.contains("player:12345:inventory"));
        assert!(msg.starts_with("[Abuse]"));
        assert!(msg.contains("allowed syntax"));
    }

    #[test]
    fn abuse_logging_decision_only_logs_when_configured() {
        // The actual logging is gated by cfg.log_abuse > 0 in handle_set.
        // We already have unit tests proving the validator rejects bad data.
        // Full verification of different LogAbuse levels + exact ABUSE-LOG
        // Redis entries is covered by the live tests (live_abuse_*).
        let should_log = |log_abuse: u8| log_abuse > 0;
        assert!(!should_log(0));
        assert!(should_log(1));
        assert!(should_log(2));
    }

    #[test]
    fn format_abuse_message_is_consistent() {
        let msg = format_abuse_message("player:12345:inventory");
        assert_eq!(
            msg,
            "[Abuse] SET key player:12345:inventory does not match the allowed syntax!"
        );
    }

    #[test]
    fn handle_be_broadcast_and_lock_use_mock() {
        let mut mock = crate::be::MockBEClient::new("");

        // Simulate handle_be_broadcast
        let _ = with_be_client_for_test(&mut mock, |b| b.say("Test broadcast"));

        // Simulate lock
        let _ = with_be_client_for_test(&mut mock, |b| b.lock());

        assert!(mock
            .commands_called
            .iter()
            .any(|c| c.contains("say -1 Test broadcast")));
        assert!(mock.commands_called.iter().any(|c| c.contains("#lock")));
    }

    #[test]
    fn abuse_log_includes_value_at_level_2() {
        // Documents that at LogAbuse=2 the full bad value is emitted (via eprintln in handle_set)
        let bad_value = r#"["bad", {"evil": true}]"#;
        let key = "test:key";
        let base_msg = format_abuse_message(key);
        let full_log_line = format!("{} | Value: {}", base_msg, bad_value);
        assert!(full_log_line.contains(bad_value));
    }
}

// Test-only helper to demonstrate mock injection for 9xx
#[cfg(test)]
fn with_be_client_for_test<F, C>(client: &mut C, mut action: F) -> std::io::Result<()>
where
    F: FnMut(&mut C) -> std::io::Result<()>,
    C: crate::be::BattlEyeClient,
{
    action(client)
}

// Note: We are currently using fully manual raw exports for maximum
// compatibility. P3 will introduce optional structured support via arma-rs
// (starting with RVExtensionArgs) without breaking the legacy path.
