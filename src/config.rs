//! Configuration loader that matches the original C++ behavior as closely as possible.
//!
//! Original search order (first file that exists wins):
//!   1. {config_path}/EpochServer.ini
//!   2. {profile_path}/EpochServer.ini
//!   3. {config_path}/epochserver.ini   (note lowercase 'e')
//!
//! If none exist → the original called exit(1) after logging.
//! For the Rust version we will return a sensible default + warning for now,
//! but we keep the same search logic.

use std::path::Path;

use config::{Config, ConfigError, File, FileFormat};

#[derive(Debug, Clone)]
pub struct EpochServerConfig {
    // [EpochServer]
    pub battleye_path: String,
    pub instance_id: String,
    pub log_abuse: u8,
    pub log_limit: u32,
    pub battleye_ip: String,
    pub battleye_port: u16,
    pub battleye_password: String,

    // [Redis]
    pub redis_ip: String,
    pub redis_port: u16,
    pub redis_password: String,
    pub redis_db: u32,

    // [SteamAPI]
    pub steam_logging: u8,
    pub steam_key: String,
    pub steam_vac_banned: bool,
    pub steam_vac_min_bans: u32,
    pub steam_vac_max_days: u32,
    pub steam_player_allow_older_than: u32,

    // Official server check (anti-fork / anti-tamper)
    pub official_check: bool,
}

impl Default for EpochServerConfig {
    fn default() -> Self {
        Self {
            battleye_path: String::new(),
            instance_id: "NA123".to_string(),
            log_abuse: 0,
            log_limit: 999,
            battleye_ip: "127.0.0.1".to_string(),
            battleye_port: 2306,
            battleye_password: String::new(),

            redis_ip: "127.0.0.1".to_string(),
            redis_port: 6379,
            redis_password: String::new(),
            redis_db: 0,

            steam_logging: 0,
            steam_key: String::new(),
            steam_vac_banned: false,
            steam_vac_min_bans: 0,
            steam_vac_max_days: 0,
            steam_player_allow_older_than: 0,
            official_check: true,
        }
    }
}

impl EpochServerConfig {
    /// Load configuration following the exact original search order.
    ///
    /// `config_path`  = directory containing the DLL (Windows) or "@epochhive" (Linux)
    /// `profile_path` = value parsed from `-profiles=` command line argument
    pub fn load(config_path: &str, profile_path: &str) -> Result<Self, ConfigError> {
        let mut builder = Config::builder();

        let mut found = false;

        // 1. {config_path}/EpochServer.ini
        let p1 = Path::new(config_path).join("EpochServer.ini");
        if p1.exists() {
            builder = builder.add_source(File::from(p1).format(FileFormat::Ini));
            found = true;
        }

        // 2. {profile_path}/EpochServer.ini
        if !found && !profile_path.is_empty() {
            let p2 = Path::new(profile_path).join("EpochServer.ini");
            if p2.exists() {
                builder = builder.add_source(File::from(p2).format(FileFormat::Ini));
                found = true;
            }
        }

        // 3. {config_path}/epochserver.ini (lowercase)
        if !found {
            let p3 = Path::new(config_path).join("epochserver.ini");
            if p3.exists() {
                builder = builder.add_source(File::from(p3).format(FileFormat::Ini));
                found = true;
            }
        }

        if !found {
            // Original behavior: fatal. We log clearly and still return defaults so the
            // extension doesn't instantly crash during early development.
            eprintln!(
                "[EpochServer] ERROR: No EpochServer.ini found in any search path!\n  \
                 Searched: {}/EpochServer.ini, {}/EpochServer.ini, {}/epochserver.ini",
                config_path, profile_path, config_path
            );
        }

        let cfg = builder
            .set_default("EpochServer.InstanceID", "NA123")?
            .set_default("EpochServer.LogAbuse", 0)?
            .set_default("EpochServer.LogLimit", 999)?
            .set_default("EpochServer.IP", "127.0.0.1")?
            .set_default("EpochServer.Port", 2306)?
            .set_default("EpochServer.Password", "")?
            .set_default("Redis.IP", "127.0.0.1")?
            .set_default("Redis.Port", 6379)?
            .set_default("Redis.Password", "")?
            .set_default("Redis.DB", 0)?
            .set_default("SteamAPI.Logging", 0)?
            .set_default("SteamAPI.Key", "")?
            .set_default("SteamAPI.VACBanned", 0)?
            .set_default("SteamAPI.VACMinimumNumberOfBans", 0)?
            .set_default("SteamAPI.VACMaximumDaysSinceLastBan", 0)?
            .set_default("SteamAPI.PlayerAllowOlderThan", 0)?
            .set_default("EpochServer.OfficialCheck", 1)?
            .build()?;

        // Helper closures
        let get_str = |key: &str, def: &str| -> String {
            cfg.get_string(key).unwrap_or_else(|_| def.to_string())
        };
        let get_u8 =
            |key: &str, def: u8| -> u8 { cfg.get_int(key).map(|v| v as u8).unwrap_or(def) };
        let get_u32 =
            |key: &str, def: u32| -> u32 { cfg.get_int(key).map(|v| v as u32).unwrap_or(def) };
        let get_bool = |key: &str, def: bool| -> bool { cfg.get_bool(key).unwrap_or(def) };
        let get_u16 =
            |key: &str, def: u16| -> u16 { cfg.get_int(key).map(|v| v as u16).unwrap_or(def) };

        let battleye_path = if profile_path.is_empty() {
            get_str("EpochServer.BattlEyePath", "")
        } else {
            get_str(
                "EpochServer.BattlEyePath",
                &format!("{}/battleye", profile_path),
            )
        };

        Ok(Self {
            battleye_path,
            instance_id: get_str("EpochServer.InstanceID", "NA123"),
            log_abuse: get_u8("EpochServer.LogAbuse", 0),
            log_limit: get_u32("EpochServer.LogLimit", 999),
            battleye_ip: get_str("EpochServer.IP", "127.0.0.1"),
            battleye_port: get_u16("EpochServer.Port", 2306),
            battleye_password: get_str("EpochServer.Password", ""),

            redis_ip: get_str("Redis.IP", "127.0.0.1"),
            redis_port: get_u16("Redis.Port", 6379),
            redis_password: get_str("Redis.Password", ""),
            redis_db: get_u32("Redis.DB", 0),

            steam_logging: get_u8("SteamAPI.Logging", 0),
            steam_key: get_str("SteamAPI.Key", ""),
            steam_vac_banned: get_bool("SteamAPI.VACBanned", false),
            steam_vac_min_bans: get_u32("SteamAPI.VACMinimumNumberOfBans", 0),
            steam_vac_max_days: get_u32("SteamAPI.VACMaximumDaysSinceLastBan", 0),
            steam_player_allow_older_than: get_u32("SteamAPI.PlayerAllowOlderThan", 0),

            official_check: get_bool("EpochServer.OfficialCheck", true),
        })
    }

    /// Convenience constructor that always succeeds (uses defaults + prints warning).
    pub fn load_or_default(config_path: &str, profile_path: &str) -> Self {
        Self::load(config_path, profile_path).unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// Helper: create a temp dir + write an INI file, run the test, then clean up.
    fn with_temp_config<F>(filename: &str, content: &str, f: F)
    where
        F: FnOnce(&str, &str),
    {
        let base = std::env::temp_dir().join(format!(
            "epochserver-config-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).expect("failed to create temp config dir");

        let ini = base.join(filename);
        fs::write(&ini, content).expect("failed to write temp INI");

        // Use the directory as both config_path and profile_path for simplicity in most tests
        let path_str = base.to_string_lossy().to_string();
        f(&path_str, &path_str);

        let _ = fs::remove_dir_all(&base);
    }

    #[test]
    fn default_config_has_reasonable_values() {
        let cfg = EpochServerConfig::default();
        assert_eq!(cfg.instance_id, "NA123");
        assert!(cfg.official_check);
    }

    #[test]
    fn load_or_default_returns_something_even_on_bad_path() {
        let cfg = EpochServerConfig::load_or_default("/nonexistent/path", "");
        assert_eq!(cfg.instance_id, "NA123");
    }

    #[test]
    fn load_reads_epochserver_ini_from_config_path() {
        let ini = r#"
[EpochServer]
InstanceID = FROM_CONFIG_PATH
LogAbuse = 2
BattlEyePath = MyBattleyeFolder
"#;
        with_temp_config("EpochServer.ini", ini, |config_path, _profile| {
            let cfg = EpochServerConfig::load(config_path, "").expect("load should succeed");
            assert_eq!(cfg.instance_id, "FROM_CONFIG_PATH");
            assert_eq!(cfg.log_abuse, 2);
            assert_eq!(cfg.battleye_path, "MyBattleyeFolder");
        });
    }

    #[test]
    fn load_uses_profile_path_when_no_top_level_ini() {
        let ini = r#"
[EpochServer]
InstanceID = FROM_PROFILE
LogLimit = 12345
"#;

        // We will put the INI only under a "profile" subdirectory
        let base = std::env::temp_dir().join(format!(
            "epochserver-config-profile-test-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();

        let config_dir = base.join("config");
        let profile_dir = base.join("myprofile");
        fs::create_dir_all(&profile_dir).unwrap();

        // No EpochServer.ini in config dir
        let ini_path = profile_dir.join("EpochServer.ini");
        fs::write(&ini_path, ini).unwrap();

        let cfg = EpochServerConfig::load(
            &config_dir.to_string_lossy(),
            &profile_dir.to_string_lossy(),
        )
        .expect("should find INI in profile path (priority 2)");

        assert_eq!(cfg.instance_id, "FROM_PROFILE");
        assert_eq!(cfg.log_limit, 12345);

        let _ = fs::remove_dir_all(&base);
    }

    // Note: The lowercase "epochserver.ini" fallback (priority 3) is difficult to
    // reliably test for *existence success* on Windows because the filesystem is
    // case-insensitive. The code path is simple and the "no file found" warning
    // path is already exercised by other tests. We accept that the exact p3 success
    // arm may stay partially uncovered on Windows.

    #[test]
    fn load_reads_redis_and_steam_sections() {
        let ini = r#"
[Redis]
IP = 10.0.0.5
Port = 6380
DB = 3

[SteamAPI]
Key = MYSTEAMKEY123
VACBanned = 1
VACMinimumNumberOfBans = 5
"#;

        with_temp_config("EpochServer.ini", ini, |config_path, _| {
            let cfg = EpochServerConfig::load(config_path, "").expect("load should succeed");
            assert_eq!(cfg.redis_ip, "10.0.0.5");
            assert_eq!(cfg.redis_port, 6380);
            assert_eq!(cfg.redis_db, 3);
            assert_eq!(cfg.steam_key, "MYSTEAMKEY123");
            assert!(cfg.steam_vac_banned);
            assert_eq!(cfg.steam_vac_min_bans, 5);
        });
    }
}
