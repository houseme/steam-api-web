// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! ISteamUserStats: achievements, game schemas and per-user / global stats.

use crate::{
    Error, Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::{StatValue, nonzero, text},
    user::validate_user_app,
    user_auth::OpaqueResponse,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

/// Access to ISteamUserStats methods.
#[derive(Debug, Clone, Copy)]
pub struct UserStats<'a>(pub(crate) &'a SteamClient);

/// Global achievement completion percentage.
#[derive(Debug, Clone, Deserialize)]
pub struct AchievementPercentage {
    /// Achievement API name.
    pub name: String,
    /// Percent of players who have unlocked the achievement.
    pub percent: f64,
}
/// Global completion percentage envelope.
#[derive(Debug, Clone, Deserialize)]
pub struct AchievementPercentages {
    /// Achievement completion rates, empty if none are configured.
    pub achievements: Vec<AchievementPercentage>,
}
/// Optional inclusive Unix timestamp range used by global stats.
#[derive(Debug, Clone, Copy, Default)]
pub struct GlobalStatsRange {
    /// First date for daily totals (uint32 Unix seconds).
    pub startdate: Option<u32>,
    /// Last date for daily totals (uint32 Unix seconds).
    pub enddate: Option<u32>,
}
/// A single aggregate stat. Unknown fields are retained because Valve does not
/// specify all daily-series response shapes in the linked reference.
#[derive(Debug, Clone, Deserialize)]
pub struct GlobalStat {
    /// Lifetime total, if returned. Strings and numbers are accepted losslessly.
    pub total: Option<StatValue>,
    /// Additional series or metadata supplied by Steam.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
/// Global stats by API name.
#[derive(Debug, Clone, Deserialize)]
pub struct GlobalStats {
    /// Aggregate statistics.
    pub globalstats: BTreeMap<String, GlobalStat>,
    /// EResult value returned by Steam (success is 1).
    pub result: i32,
}
/// Current online player count.
#[derive(Debug, Clone, Deserialize)]
pub struct CurrentPlayers {
    /// Players connected to Steam; offline players are excluded.
    pub player_count: u64,
    /// EResult value returned by Steam (success is 1).
    pub result: i32,
}
/// A user's achievement state.
#[derive(Debug, Clone, Deserialize)]
pub struct PlayerAchievement {
    /// Achievement API name.
    pub apiname: String,
    /// Steam wire value (0 locked, 1 unlocked).
    pub achieved: u32,
    /// Unix unlock timestamp (0 when locked).
    pub unlocktime: u64,
    /// Localized name, when requested and available.
    pub name: Option<String>,
    /// Localized description, when available.
    pub description: Option<String>,
}
/// Per-user achievement data.
#[derive(Debug, Clone, Deserialize)]
pub struct PlayerAchievements {
    /// User's Steam ID.
    #[serde(rename = "steamID", alias = "steamid")]
    pub steamid: SteamId,
    /// Game display name, when returned.
    #[serde(rename = "gameName")]
    pub game_name: Option<String>,
    /// Achievements; Steam can omit this field for games with none.
    #[serde(default)]
    pub achievements: Vec<PlayerAchievement>,
}
/// Definition of a game statistic. Web reads preserve integer and float values;
/// the documented Web write method specifically accepts uint32 values.
#[derive(Debug, Clone, Deserialize)]
pub struct StatSchema {
    /// Stat API name.
    pub name: String,
    /// Configured default value.
    pub defaultvalue: StatValue,
    /// Display name, when supplied.
    #[serde(rename = "displayName")]
    pub display_name: Option<String>,
}
/// Definition of a game achievement.
#[derive(Debug, Clone, Deserialize)]
pub struct AchievementSchema {
    /// Achievement API name.
    pub name: String,
    /// Default state.
    pub defaultvalue: StatValue,
    /// Display name.
    #[serde(rename = "displayName")]
    pub display_name: String,
    /// Hidden marker (0/1).
    pub hidden: u32,
    /// Description, omitted for some hidden achievements.
    pub description: Option<String>,
    /// Unlocked icon URL.
    pub icon: String,
    /// Locked icon URL.
    pub icongray: String,
}
/// Stats and achievements configured for the game.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct AvailableGameStats {
    /// Stats, possibly absent for games that only have achievements.
    #[serde(default)]
    pub stats: Vec<StatSchema>,
    /// Achievements, possibly absent for games that only have stats.
    #[serde(default)]
    pub achievements: Vec<AchievementSchema>,
}
/// A game's complete public schema.
#[derive(Debug, Clone, Deserialize)]
pub struct GameSchema {
    /// Display name (some apps return no name).
    #[serde(rename = "gameName")]
    pub game_name: Option<String>,
    /// Schema version as returned by Steam.
    #[serde(rename = "gameVersion")]
    pub game_version: Option<String>,
    /// Configured stats / achievements, absent for games with neither.
    #[serde(rename = "availableGameStats", default)]
    pub available_game_stats: AvailableGameStats,
}
/// An individual user stat.
#[derive(Debug, Clone, Deserialize)]
pub struct UserStat {
    /// Stat API name.
    pub name: String,
    /// Numeric value without unconditional conversion to floating point.
    pub value: StatValue,
}
/// Achievement state returned by GetUserStatsForGame.
#[derive(Debug, Clone, Deserialize)]
pub struct UserStatAchievement {
    /// Achievement API name.
    pub name: String,
    /// Wire state (0/1).
    pub achieved: u32,
}
/// Per-user stats and achievements.
#[derive(Debug, Clone, Deserialize)]
pub struct UserGameStats {
    /// User's Steam ID.
    #[serde(rename = "steamID", alias = "steamid")]
    pub steamid: SteamId,
    /// Game display name, if returned.
    #[serde(rename = "gameName")]
    pub game_name: Option<String>,
    /// Stats, possibly absent for an achievement-only game.
    #[serde(default)]
    pub stats: Vec<UserStat>,
    /// Achievement states, possibly absent for a stat-only game.
    #[serde(default)]
    pub achievements: Vec<UserStatAchievement>,
}
/// One name/value pair for the documented uint32 Web write API.
#[derive(Debug, Clone, Copy)]
pub struct StatUpdate<'a> {
    /// Stat or achievement API name.
    pub name: &'a str,
    /// uint32 value (achievement state: 0 or 1).
    pub value: u32,
}

impl UserStats<'_> {
    /// Get global achievement percentages (v2). Anonymous on the public host.
    pub async fn get_global_achievement_percentages_for_app(
        &self,
        gameid: u64,
    ) -> Result<AchievementPercentages> {
        nonzero(gameid, "gameid must not be zero")?;
        self.0
            .call(
                Endpoint::get(
                    "ISteamUserStats/GetGlobalAchievementPercentagesForApp/v2/",
                    Access::Anonymous,
                ),
                Params::new().with("gameid", gameid),
                &["achievementpercentages"],
            )
            .await
    }
    /// Get global stats with indexed `name[i]` parameters and optional date range.
    pub async fn get_global_stats_for_game(
        &self,
        appid: u32,
        names: &[&str],
        range: GlobalStatsRange,
    ) -> Result<GlobalStats> {
        nonzero(appid.into(), "appid must not be zero")?;
        let count = validate_names(names.iter().copied(), names.len())?;
        if range
            .startdate
            .zip(range.enddate)
            .is_some_and(|(start, end)| start > end)
        {
            return Err(Error::InvalidParameter("startdate must not exceed enddate"));
        }
        let mut params = Params::new().with("appid", appid).with("count", count);
        for (index, name) in names.iter().enumerate() {
            params.push_indexed("name", index, name);
        }
        if let Some(start) = range.startdate {
            params.push("startdate", start);
        }
        if let Some(end) = range.enddate {
            params.push("enddate", end);
        }
        self.0
            .call(
                Endpoint::get(
                    "ISteamUserStats/GetGlobalStatsForGame/v1/",
                    Access::Publisher,
                ),
                params,
                &["response"],
            )
            .await
    }
    /// Count online players. Anonymous on the public host.
    pub async fn get_number_of_current_players(&self, appid: u32) -> Result<CurrentPlayers> {
        nonzero(appid.into(), "appid must not be zero")?;
        self.0
            .call(
                Endpoint::get(
                    "ISteamUserStats/GetNumberOfCurrentPlayers/v1/",
                    Access::Anonymous,
                ),
                Params::new().with("appid", appid),
                &["response"],
            )
            .await
    }
    /// Read user achievements, optionally localized with Steam language names.
    pub async fn get_player_achievements(
        &self,
        steamid: impl Into<SteamId>,
        appid: u32,
        language: Option<&str>,
    ) -> Result<PlayerAchievements> {
        let steamid = steamid.into();
        validate_user_app(steamid, appid)?;
        let mut params = Params::new().with("steamid", steamid).with("appid", appid);
        add_language(&mut params, language)?;
        self.0
            .call(
                Endpoint::get("ISteamUserStats/GetPlayerAchievements/v1/", Access::User),
                params,
                &["playerstats"],
            )
            .await
    }
    /// Read game stats and achievement definitions (v2).
    pub async fn get_schema_for_game(
        &self,
        appid: u32,
        language: Option<&str>,
    ) -> Result<GameSchema> {
        nonzero(appid.into(), "appid must not be zero")?;
        let mut params = Params::new().with("appid", appid);
        add_language(&mut params, language)?;
        self.0
            .call(
                Endpoint::get("ISteamUserStats/GetSchemaForGame/v2/", Access::User),
                params,
                &["game"],
            )
            .await
    }
    /// Read per-user stats and achievement states (v2).
    pub async fn get_user_stats_for_game(
        &self,
        steamid: impl Into<SteamId>,
        appid: u32,
    ) -> Result<UserGameStats> {
        let steamid = steamid.into();
        validate_user_app(steamid, appid)?;
        self.0
            .call(
                Endpoint::get("ISteamUserStats/GetUserStatsForGame/v2/", Access::User),
                Params::new().with("steamid", steamid).with("appid", appid),
                &["playerstats"],
            )
            .await
    }
    /// Write uint32 stats / achievements. No retry is performed after a timeout,
    /// transport failure or HTTP error: the server may have already applied it.
    /// Names must be nonempty and unique. Check Steam's write result explicitly;
    /// the official reference does not specify its success response schema.
    pub async fn set_user_stats_for_game(
        &self,
        steamid: impl Into<SteamId>,
        appid: u32,
        updates: &[StatUpdate<'_>],
    ) -> Result<OpaqueResponse> {
        let steamid = steamid.into();
        validate_user_app(steamid, appid)?;
        let count = validate_names(updates.iter().map(|u| u.name), updates.len())?;
        let mut params = Params::new()
            .with("steamid", steamid)
            .with("appid", appid)
            .with("count", count);
        for (index, update) in updates.iter().enumerate() {
            params.push_indexed("name", index, update.name);
            params.push_indexed("value", index, update.value);
        }
        self.0
            .call_opaque(
                Endpoint::post("ISteamUserStats/SetUserStatsForGame/v1/", Access::Publisher),
                params,
            )
            .await
    }
}
fn add_language(params: &mut Params, language: Option<&str>) -> Result<()> {
    if let Some(language) = language {
        text(language, "language must not be empty or contain NUL")?;
        params.push("l", language);
    }
    Ok(())
}
fn validate_names<'a>(names: impl Iterator<Item = &'a str>, len: usize) -> Result<u32> {
    let count = u32::try_from(len).map_err(|_| Error::InvalidParameter("too many stat names"))?;
    if count == 0 {
        return Err(Error::InvalidParameter(
            "at least one stat name is required",
        ));
    }
    let mut seen = HashSet::new();
    for name in names {
        text(name, "stat name must not be empty or contain NUL")?;
        if !seen.insert(name) {
            return Err(Error::InvalidParameter("stat names must be unique"));
        }
    }
    Ok(count)
}
