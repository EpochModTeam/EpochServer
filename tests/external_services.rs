//! Opt-in smoke tests for real external services.
//!
//! These tests skip by default. They are useful before releases when a developer
//! has a real BattlEye RCon endpoint or Steam Web API key available.

use epochserver_x64::be::BEClient;
use epochserver_x64::steam::{SteamAPI, SteamConfig};

#[test]
fn real_battleye_rcon_smoke_when_env_configured() {
    let ip = match std::env::var("EPOCH_BE_IP") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            eprintln!("Skipping real BE smoke test: EPOCH_BE_IP is not set");
            return;
        }
    };
    let port = std::env::var("EPOCH_BE_PORT")
        .ok()
        .and_then(|v| v.parse::<u16>().ok())
        .unwrap_or(2306);
    let password = match std::env::var("EPOCH_BE_PASSWORD") {
        Ok(v) if !v.is_empty() => v,
        _ => {
            eprintln!("Skipping real BE smoke test: EPOCH_BE_PASSWORD is not set");
            return;
        }
    };

    let mut client = BEClient::new(&ip, port).expect("create BE client");
    assert!(client.login(&password).expect("BE login request"));
    let players = client
        .execute_command("players")
        .expect("BE players command");
    println!("BE players response: {}", players);
}

#[test]
fn real_steam_api_smoke_when_env_configured() {
    let key = match std::env::var("EPOCH_STEAM_API_KEY") {
        Ok(v) if !v.trim().is_empty() => v,
        _ => {
            eprintln!("Skipping real Steam smoke test: EPOCH_STEAM_API_KEY is not set");
            return;
        }
    };
    let steamid =
        std::env::var("EPOCH_STEAM_TEST_ID").unwrap_or_else(|_| "76561197960435530".to_string());

    let api = SteamAPI::new(SteamConfig {
        key,
        logging: 0,
        vac_banned: false,
        vac_min_bans: 0,
        vac_max_days: 0,
        player_allow_older_than: 0,
    });

    let rt = tokio::runtime::Runtime::new().expect("runtime");
    let bans = rt
        .block_on(api.get_player_bans(&steamid))
        .expect("Steam bans response");
    assert_eq!(bans.steam_id, steamid);

    let summary = rt
        .block_on(api.get_player_summary(&steamid))
        .expect("Steam summary response");
    assert_eq!(summary.steamid, steamid);
}
