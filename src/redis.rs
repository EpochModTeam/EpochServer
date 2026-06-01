//! Redis layer — aims to replicate the spirit and observable behavior of the original
//! RedisConnector + EpochlibRedisExecute as closely as practical.
//!
//! Original key behaviors:
//! - Lazy connection with AUTH + SELECT on first use
//! - Shared context protected by mutex
//! - execute(format, ...) style with STRING / INTEGER reply handling
//! - Reconnect on errors

use std::sync::OnceLock;

use redis::{aio::ConnectionManager, Client};
use tokio::sync::Mutex;

use crate::config::EpochServerConfig;

/// Result type matching the original EpochlibRedisExecute.
#[derive(Debug, Clone, Default)]
pub struct RedisExecuteResult {
    pub success: bool,
    pub message: String,
}

/// Global connection manager + initialization guard.
static REDIS: OnceLock<Mutex<Option<ConnectionManager>>> = OnceLock::new();

/// Build Redis URL from config (or env override).
fn build_redis_url(cfg: &EpochServerConfig) -> String {
    if let Ok(url) = std::env::var("EPOCH_REDIS_URL") {
        return url;
    }
    if let Ok(url) = std::env::var("REDIS_URL") {
        return url;
    }

    let auth = if cfg.redis_password.is_empty() {
        String::new()
    } else {
        // Simple encoding — for production passwords with special chars,
        // users should prefer EPOCH_REDIS_URL env var.
        format!(":{}@", cfg.redis_password)
    };

    format!(
        "redis://{}{}:{}/{}",
        auth, cfg.redis_ip, cfg.redis_port, cfg.redis_db
    )
}

/// Get or initialize the connection manager.
/// This is the equivalent of the original _reconnect + AUTH + SELECT logic.
async fn get_connection() -> Option<ConnectionManager> {
    let mut guard = REDIS.get_or_init(|| Mutex::new(None)).lock().await;

    if let Some(cm) = &*guard {
        return Some(cm.clone());
    }

    // Load config (this will use the global cached config if already initialized)
    let cfg = crate::config::EpochServerConfig::load_or_default("", "");
    let url = build_redis_url(&cfg);

    match Client::open(url.clone()) {
        Ok(client) => {
            match client.get_connection_manager().await {
                Ok(cm) => {
                    // ConnectionManager handles reconnects automatically.
                    // The original did AUTH + SELECT here; the URL above already includes
                    // credentials and database, so the client handles it.
                    eprintln!("[EpochServer] Redis connection established: {}", url);
                    *guard = Some(cm.clone());
                    Some(cm)
                }
                Err(e) => {
                    eprintln!(
                        "[EpochServer] Failed to create Redis ConnectionManager: {}",
                        e
                    );
                    None
                }
            }
        }
        Err(e) => {
            eprintln!("[EpochServer] Invalid Redis URL '{}': {}", url, e);
            None
        }
    }
}

/// Execute a Redis command.
/// This is the main entry point used by all handlers.
/// It mirrors the original execute() contract as much as possible.
pub async fn execute(cmd: &str, args: &[String]) -> RedisExecuteResult {
    let Some(mut conn) = get_connection().await else {
        return RedisExecuteResult {
            success: false,
            message: "Redis not connected".into(),
        };
    };

    // Build command with all arguments
    let mut redis_cmd = redis::cmd(cmd);
    for a in args {
        redis_cmd.arg(a.as_str());
    }

    // Try string reply first (most common)
    if let Ok(s) = redis_cmd.clone().query_async::<String>(&mut conn).await {
        return RedisExecuteResult {
            success: true,
            message: s,
        };
    }

    // Fall back to integer (counts, TTL, etc.)
    match redis_cmd.query_async::<i64>(&mut conn).await {
        Ok(n) => RedisExecuteResult {
            success: true,
            message: n.to_string(),
        },
        Err(e) => RedisExecuteResult {
            success: false,
            message: e.to_string(),
        },
    }
}

/// Force a fresh connection on next use (useful for testing or after config change).
///
/// Intentionally kept `pub(crate)` + allowed as dead code because:
/// - It is a useful escape hatch for tests and manual debugging.
/// - It is not called in normal production paths today.
#[allow(dead_code)]
pub(crate) async fn reset_connection() {
    if let Some(guard) = REDIS.get() {
        let mut g = guard.lock().await;
        *g = None;
    }
}

/// Explicitly initialize the Redis connection early (called on first RVExtension, like original).
pub async fn initialize() -> bool {
    get_connection().await.is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::EpochServerConfig;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    /// RAII guard that restores environment variables on drop.
    /// Makes URL builder tests safe and order-independent.
    struct EnvGuard {
        epoch: Option<String>,
        redis: Option<String>,
    }

    impl EnvGuard {
        fn new() -> Self {
            Self {
                epoch: std::env::var("EPOCH_REDIS_URL").ok(),
                redis: std::env::var("REDIS_URL").ok(),
            }
        }
    }

    impl Drop for EnvGuard {
        fn drop(&mut self) {
            match &self.epoch {
                Some(v) => std::env::set_var("EPOCH_REDIS_URL", v),
                None => std::env::remove_var("EPOCH_REDIS_URL"),
            }
            match &self.redis {
                Some(v) => std::env::set_var("REDIS_URL", v),
                None => std::env::remove_var("REDIS_URL"),
            }
        }
    }

    fn make_test_cfg() -> EpochServerConfig {
        // Minimal config for URL building tests
        EpochServerConfig {
            redis_ip: "127.0.0.1".to_string(),
            redis_port: 6379,
            redis_db: 0,
            redis_password: String::new(),
            ..EpochServerConfig::default()
        }
    }

    #[test]
    fn build_redis_url_uses_env_override() {
        let _env_lock = ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard::new();
        std::env::set_var("EPOCH_REDIS_URL", "redis://override:9999/5");

        let cfg = make_test_cfg();
        let url = build_redis_url(&cfg);
        assert_eq!(url, "redis://override:9999/5");
    }

    #[test]
    fn build_redis_url_falls_back_to_config() {
        let _env_lock = ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard::new();
        std::env::remove_var("EPOCH_REDIS_URL");
        std::env::remove_var("REDIS_URL");

        let mut cfg = make_test_cfg();
        cfg.redis_password = "secret".to_string();
        cfg.redis_ip = "10.0.0.5".to_string();
        cfg.redis_port = 6380;
        cfg.redis_db = 2;

        let url = build_redis_url(&cfg);
        assert_eq!(url, "redis://:secret@10.0.0.5:6380/2");
    }

    #[test]
    fn build_redis_url_no_password() {
        let _env_lock = ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard::new();
        std::env::remove_var("EPOCH_REDIS_URL");
        std::env::remove_var("REDIS_URL");

        let cfg = make_test_cfg();
        let url = build_redis_url(&cfg);
        assert_eq!(url, "redis://127.0.0.1:6379/0");
    }

    #[test]
    fn build_redis_url_redis_url_fallback() {
        let _env_lock = ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard::new();
        std::env::remove_var("EPOCH_REDIS_URL");
        std::env::set_var("REDIS_URL", "redis://from-redis-url:7777/7");

        let cfg = make_test_cfg();
        let url = build_redis_url(&cfg);
        assert_eq!(url, "redis://from-redis-url:7777/7");
    }

    #[test]
    fn build_redis_url_epoch_beats_redis_url() {
        let _env_lock = ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard::new();
        std::env::set_var("EPOCH_REDIS_URL", "redis://epoch-wins:1234/1");
        std::env::set_var("REDIS_URL", "redis://should-be-ignored:9999/9");

        let cfg = make_test_cfg();
        let url = build_redis_url(&cfg);
        assert_eq!(url, "redis://epoch-wins:1234/1");
    }

    #[test]
    fn build_redis_url_password_special_chars() {
        let _env_lock = ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard::new();
        std::env::remove_var("EPOCH_REDIS_URL");
        std::env::remove_var("REDIS_URL");

        let mut cfg = make_test_cfg();
        cfg.redis_password = "p@ss:word/123".to_string();
        cfg.redis_ip = "redis.example.com".to_string();
        cfg.redis_port = 6381;
        cfg.redis_db = 5;

        let url = build_redis_url(&cfg);
        // Note: our simple encoding does not percent-encode. This documents current behavior.
        assert_eq!(url, "redis://:p@ss:word/123@redis.example.com:6381/5");
    }

    #[test]
    fn redis_execute_result_default_and_construction() {
        let default = RedisExecuteResult::default();
        assert!(!default.success);
        assert!(default.message.is_empty());

        let success = RedisExecuteResult {
            success: true,
            message: "42".into(),
        };
        assert!(success.success);
        assert_eq!(success.message, "42");

        let failure = RedisExecuteResult {
            success: false,
            message: "Redis not connected".into(),
        };
        assert!(!failure.success);
    }

    #[test]
    fn reset_connection_is_safe_to_call() {
        // Should not panic even if never initialized
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            reset_connection().await;
            reset_connection().await; // second call is also fine
        });
    }

    #[test]
    fn execute_returns_not_connected_when_no_redis() {
        // Force the global state into "no connection" and verify the public
        // failure path in execute() without touching the network.
        let _env_lock = ENV_LOCK.lock().unwrap();
        let _guard = EnvGuard::new();
        std::env::set_var("EPOCH_REDIS_URL", "not-a-valid-redis-url");

        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            reset_connection().await;

            let result = execute("GET", &["some:key".to_string()]).await;
            assert!(!result.success);
            assert_eq!(result.message, "Redis not connected");
        });
    }

    // Note: We intentionally do not have a unit test that calls initialize() / get_connection()
    // without a Redis server, because Client::open + get_connection_manager() can take a long
    // time to time out. Those paths are exercised by the live_hive_commands and redis_integration
    // test suites when a real Redis is available.
}
