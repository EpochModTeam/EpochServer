//! Steam Web API client for player validation (VAC bans, account age).
//! Used by the 001 command.

use reqwest::Client;
use serde::Deserialize;

#[derive(Debug, Clone)]
pub struct SteamConfig {
    pub key: String,
    #[allow(dead_code)]
    // Present in EpochServer.ini (SteamAPI.Logging) and loaded for fidelity with the original,
    // but not yet wired into actual logging behavior in the Steam client.
    // Kept for future use and to avoid breaking existing INI files.
    pub logging: u8,
    pub vac_banned: bool,
    pub vac_min_bans: u32,
    pub vac_max_days: u32,
    pub player_allow_older_than: u32,
}

#[derive(Deserialize, Debug)]
struct PlayerBansResponse {
    players: Vec<PlayerBanInfo>,
}

#[derive(Deserialize, Debug)]
pub struct PlayerBanInfo {
    #[serde(rename = "SteamId")]
    pub steam_id: String,
    #[serde(rename = "VACBanned")]
    pub vac_banned: bool,
    #[serde(rename = "NumberOfVACBans")]
    pub number_of_vac_bans: u32,
    #[serde(rename = "DaysSinceLastBan")]
    pub days_since_last_ban: u32,
}

#[derive(Deserialize, Debug)]
struct PlayerSummariesResponse {
    response: PlayerSummaries,
}

#[derive(Deserialize, Debug)]
struct PlayerSummaries {
    players: Vec<PlayerSummary>,
}

#[derive(Deserialize, Debug)]
pub struct PlayerSummary {
    pub steamid: String,
    pub timecreated: Option<u64>,
}

pub struct SteamAPI {
    client: Client,
    config: SteamConfig,
}

impl SteamAPI {
    pub fn new(config: SteamConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    pub async fn get_player_bans(&self, steamid: &str) -> Option<PlayerBanInfo> {
        if self.config.key.is_empty() {
            return None;
        }

        let url = format!(
            "https://api.steampowered.com/ISteamUser/GetPlayerBans/v1/?key={}&steamids={}",
            self.config.key, steamid
        );

        match self.client.get(&url).send().await {
            Ok(resp) => resp
                .json::<PlayerBansResponse>()
                .await
                .ok()
                .and_then(|r| r.players.into_iter().next()),
            Err(_) => None,
        }
    }

    pub async fn get_player_summary(&self, steamid: &str) -> Option<PlayerSummary> {
        if self.config.key.is_empty() {
            return None;
        }

        let url = format!(
            "https://api.steampowered.com/ISteamUser/GetPlayerSummaries/v0002/?key={}&steamids={}",
            self.config.key, steamid
        );

        match self.client.get(&url).send().await {
            Ok(resp) => resp
                .json::<PlayerSummariesResponse>()
                .await
                .ok()
                .and_then(|r| r.response.players.into_iter().next()),
            Err(_) => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_config(key: &str) -> SteamConfig {
        SteamConfig {
            key: key.to_string(),
            logging: 0,
            vac_banned: true,
            vac_min_bans: 1,
            vac_max_days: 365,
            player_allow_older_than: 86_400,
        }
    }

    #[test]
    fn empty_api_key_short_circuits_bans_request() {
        let api = SteamAPI::new(test_config(""));
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let result = rt.block_on(api.get_player_bans("76561197960435530"));
        assert!(result.is_none());
    }

    #[test]
    fn empty_api_key_short_circuits_summary_request() {
        let api = SteamAPI::new(test_config(""));
        let rt = tokio::runtime::Runtime::new().expect("runtime");
        let result = rt.block_on(api.get_player_summary("76561197960435530"));
        assert!(result.is_none());
    }
}
