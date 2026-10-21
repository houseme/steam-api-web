// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! ISteamUser: users, ownership, prices, account deletions and groups.

use crate::{
    Error, Result, SteamClient, SteamId,
    client::{Access, Endpoint, param},
    types::{nonzero, text, u64_string},
    user_auth::OpaqueResponse,
};
use serde::Deserialize;
use serde_json::Value;
use std::collections::BTreeMap;

/// Access to ISteamUser methods.
#[derive(Debug, Clone, Copy)]
pub struct User<'a>(pub(crate) &'a SteamClient);

/// A game license. `ownsapp` is the official active-license indicator.
#[derive(Debug, Clone, Deserialize)]
pub struct AppOwnership {
    /// Active license (including temporary/shared access).
    pub ownsapp: bool,
    /// Permanent purchase rather than shared or temporary access.
    pub permanent: bool,
    /// Acquisition timestamp, returned in Steam's original representation.
    pub timestamp: String,
    /// Expiry timestamp, or `never`. Optional for older publisher v4 replies.
    pub timeexpires: Option<String>,
    /// Actual owner; zero can occur for canceled licenses.
    pub ownersteamid: SteamId,
    /// PC Café access.
    pub sitelicense: bool,
    /// Self-canceled license.
    pub usercanceled: bool,
    /// Timed trial indicator, when supplied.
    pub timedtrial: Option<bool>,
}
/// An app and its associated license.
#[derive(Debug, Clone, Deserialize)]
pub struct PublisherAppOwnership {
    /// Publisher application ID.
    pub appid: u32,
    /// License details.
    #[serde(flatten)]
    pub ownership: AppOwnership,
}
/// All owned or self-canceled apps covered by the publisher key.
#[derive(Debug, Clone, Deserialize)]
pub struct PublisherAppOwnerships {
    /// Empty or omitted if the user owns no publisher apps.
    #[serde(default)]
    pub apps: Vec<PublisherAppOwnership>,
}
/// Publisher ownership versions present in the reference and its change history.
#[derive(Debug, Clone, Copy, Default)]
pub enum PublisherOwnershipVersion {
    /// URL explicitly printed in the reference (default).
    #[default]
    V4,
    /// Version listed in the change history as adding expiry timestamps.
    V5,
}
/// Price in minor currency units; no floating point currency conversion.
#[derive(Debug, Clone, Deserialize)]
pub struct AppPrice {
    /// Purchasing package ID.
    pub packageid: u32,
    /// Currency code such as USD.
    pub currency: String,
    /// Undiscounted price in minor units.
    pub initial_price: u64,
    /// Discounted price in minor units.
    pub final_price: u64,
    /// Discount percentage.
    pub discount_percent: u32,
}
/// A friend's relationship to the requested user.
#[derive(Debug, Clone, Deserialize)]
pub struct Friend {
    /// Friend's Steam ID.
    pub steamid: SteamId,
    /// Relationship type as returned by Steam.
    pub relationship: String,
    /// Friendship start time (Unix seconds).
    pub friend_since: u64,
}
/// Friend list envelope.
#[derive(Debug, Clone, Deserialize)]
pub struct FriendList {
    /// Friends; private lists produce HTTP 401 rather than an empty list.
    pub friends: Vec<Friend>,
}
/// A detailed app-specific ban, when returned.
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "PascalCase")]
pub struct Ban {
    /// First app in the banned range.
    pub app_id_min: u32,
    /// Last app in the banned range.
    pub app_id_max: u32,
    /// Ban start in Unix seconds.
    pub ban_start_time: u64,
    /// Steam ban type, kept numeric for future compatibility.
    pub ban_type: u32,
    /// Issuing provider ID.
    pub ban_provider_id: u32,
}
/// Ban summary with explicit wire names (notably VACBanned and SteamId).
#[derive(Debug, Clone, Deserialize)]
pub struct PlayerBans {
    /// User's Steam ID.
    #[serde(rename = "SteamId")]
    pub steamid: SteamId,
    /// Community ban flag.
    #[serde(rename = "CommunityBanned")]
    pub community_banned: bool,
    /// VAC ban flag.
    #[serde(rename = "VACBanned")]
    pub vac_banned: bool,
    /// Count of VAC bans.
    #[serde(rename = "NumberOfVACBans")]
    pub number_of_vac_bans: u32,
    /// Days since last ban.
    #[serde(rename = "DaysSinceLastBan")]
    pub days_since_last_ban: u32,
    /// Count of game bans.
    #[serde(rename = "NumberOfGameBans")]
    pub number_of_game_bans: u32,
    /// Economy restriction, retained as text for unknown future values.
    #[serde(rename = "EconomyBan")]
    pub economy_ban: String,
    /// Details are absent for many users.
    #[serde(default)]
    pub bans: Vec<Ban>,
}
/// Public user information. Private/profile-dependent fields remain optional.
#[derive(Debug, Clone, Deserialize)]
pub struct PlayerSummary {
    /// User's Steam ID.
    pub steamid: SteamId,
    /// Visibility level; kept numeric to preserve future values.
    pub communityvisibilitystate: u32,
    /// Persona name.
    pub personaname: String,
    /// Profile URL.
    pub profileurl: String,
    /// Small avatar URL.
    pub avatar: String,
    /// Medium avatar URL.
    pub avatarmedium: String,
    /// Full avatar URL.
    pub avatarfull: String,
    /// Profile configuration state, when visible.
    pub profilestate: Option<u32>,
    /// Persona presence, when visible.
    pub personastate: Option<u32>,
    /// Last logoff Unix timestamp, when visible.
    pub lastlogoff: Option<u64>,
    /// Real name, when shared by the user.
    pub realname: Option<String>,
    /// Account creation Unix timestamp, when visible.
    pub timecreated: Option<u64>,
    /// Fields added by Steam or visible only for some users.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}
/// Group membership; gid is a group account ID rather than a user SteamID64.
#[derive(Debug, Clone, Deserialize)]
pub struct UserGroup {
    /// Group ID represented as a number or decimal string.
    #[serde(deserialize_with = "u64_string")]
    pub gid: u64,
}
/// Profile types accepted by ResolveVanityURL.
#[derive(Debug, Clone, Copy, Default)]
pub enum VanityUrlType {
    /// Individual profile (1).
    #[default]
    Individual,
    /// Community group (2).
    Group,
    /// Official game group (3).
    OfficialGameGroup,
}
impl VanityUrlType {
    fn value(self) -> u32 {
        match self {
            Self::Individual => 1,
            Self::Group => 2,
            Self::OfficialGameGroup => 3,
        }
    }
}

#[derive(Deserialize)]
struct Players<T> {
    players: Vec<T>,
}
#[derive(Deserialize)]
struct Groups {
    groups: Vec<UserGroup>,
}

impl User<'_> {
    /// Check whether a user has an active license (v4).
    pub async fn check_app_ownership(
        &self,
        steamid: impl Into<SteamId>,
        appid: u32,
    ) -> Result<AppOwnership> {
        let steamid = steamid.into();
        validate_user_app(steamid, appid)?;
        self.0
            .call(
                Endpoint::get("ISteamUser/CheckAppOwnership/v4/", Access::Publisher),
                vec![param("steamid", steamid), param("appid", appid)],
                &["appownership"],
            )
            .await
    }
    /// Get prices for 1..=100 nonzero app IDs in the user's purchasing region.
    pub async fn get_app_price_info(
        &self,
        steamid: impl Into<SteamId>,
        appids: &[u32],
    ) -> Result<BTreeMap<u32, AppPrice>> {
        let steamid = steamid.into();
        nonzero(steamid.0, "steamid must not be zero")?;
        validate_batch(appids.len())?;
        for &id in appids {
            nonzero(id.into(), "appid must not be zero")?;
        }
        self.0
            .call(
                Endpoint::get("ISteamUser/GetAppPriceInfo/v1/", Access::Publisher),
                vec![
                    param("steamid", steamid),
                    param("appids", comma_list(appids)),
                ],
                &["price_info"],
            )
            .await
    }
    /// Fetch one page of deleted accounts. Start with rowversion 0 and persist
    /// Steam's returned cursor only after handling the page successfully.
    /// The official reference omits its response schema, so retain the full JSON
    /// rather than inventing a typed pagination envelope or termination rule.
    pub async fn get_deleted_steam_ids(&self, rowversion: u64) -> Result<OpaqueResponse> {
        self.0
            .call(
                Endpoint::get("ISteamUser/GetDeletedSteamIDs/v1/", Access::Publisher),
                vec![param("rowversion", rowversion)],
                &[],
            )
            .await
    }
    /// Read friends. A private friend list produces Error::Http with status 401.
    pub async fn get_friend_list(
        &self,
        steamid: impl Into<SteamId>,
        relationship: Option<&str>,
    ) -> Result<FriendList> {
        let steamid = steamid.into();
        nonzero(steamid.0, "steamid must not be zero")?;
        let mut params = vec![param("steamid", steamid)];
        if let Some(value) = relationship {
            text(value, "relationship must not be empty or contain NUL")?;
            params.push(param("relationship", value));
        }
        self.0
            .call(
                Endpoint::get("ISteamUser/GetFriendList/v1/", Access::User),
                params,
                &["friendslist"],
            )
            .await
    }
    /// Read ban summaries. This client bounds each batch to 100 IDs.
    pub async fn get_player_bans(&self, steamids: &[SteamId]) -> Result<Vec<PlayerBans>> {
        validate_steamids(steamids)?;
        let response: Players<PlayerBans> = self
            .0
            .call(
                Endpoint::get("ISteamUser/GetPlayerBans/v1/", Access::User),
                vec![param("steamids", comma_list(steamids))],
                &[],
            )
            .await?;
        Ok(response.players)
    }
    /// Read public profile summaries, for 1..=100 IDs. Steam may omit unknown IDs.
    pub async fn get_player_summaries(&self, steamids: &[SteamId]) -> Result<Vec<PlayerSummary>> {
        validate_steamids(steamids)?;
        let response: Players<PlayerSummary> = self
            .0
            .call(
                Endpoint::get("ISteamUser/GetPlayerSummaries/v2/", Access::User),
                vec![param("steamids", comma_list(steamids))],
                &["response"],
            )
            .await?;
        Ok(response.players)
    }
    /// Split a larger profile request into sequential batches of at most 100.
    /// Returns no partial result on failure. Each batch has its own operation
    /// deadline; use an outer timeout for a deadline across the full batch set.
    pub async fn get_player_summaries_batched(
        &self,
        steamids: &[SteamId],
    ) -> Result<Vec<PlayerSummary>> {
        for id in steamids {
            nonzero(id.0, "steamid must not be zero")?;
        }
        let mut players = Vec::new();
        for chunk in steamids.chunks(100) {
            players.extend(self.get_player_summaries(chunk).await?);
        }
        Ok(players)
    }
    /// Read all publisher licenses using v4, the URL printed in the reference.
    pub async fn get_publisher_app_ownership(
        &self,
        steamid: impl Into<SteamId>,
    ) -> Result<PublisherAppOwnerships> {
        self.get_publisher_app_ownership_version(steamid, PublisherOwnershipVersion::V4)
            .await
    }
    /// Explicit version selection for the reference's v4/v5 inconsistency.
    /// v5 adds expiry according to its change history; it requires live publisher
    /// validation before relying on that additional field.
    pub async fn get_publisher_app_ownership_version(
        &self,
        steamid: impl Into<SteamId>,
        version: PublisherOwnershipVersion,
    ) -> Result<PublisherAppOwnerships> {
        let steamid = steamid.into();
        nonzero(steamid.0, "steamid must not be zero")?;
        let path = match version {
            PublisherOwnershipVersion::V4 => "ISteamUser/GetPublisherAppOwnership/v4/",
            PublisherOwnershipVersion::V5 => "ISteamUser/GetPublisherAppOwnership/v5/",
        };
        self.0
            .call(
                Endpoint::get(path, Access::Publisher),
                vec![param("steamid", steamid)],
                &["appownership"],
            )
            .await
    }
    /// Read the user's group memberships.
    pub async fn get_user_group_list(&self, steamid: impl Into<SteamId>) -> Result<Vec<UserGroup>> {
        let steamid = steamid.into();
        nonzero(steamid.0, "steamid must not be zero")?;
        let response: Groups = self
            .0
            .call(
                Endpoint::get("ISteamUser/GetUserGroupList/v1/", Access::Publisher),
                vec![param("steamid", steamid)],
                &["response"],
            )
            .await?;
        Ok(response.groups)
    }
    /// Resolve a vanity name. A legitimate no-match result (Steam success=42)
    /// is returned as None. Transport, authorization and malformed responses fail.
    pub async fn resolve_vanity_url(
        &self,
        vanity: &str,
        kind: VanityUrlType,
    ) -> Result<Option<SteamId>> {
        const PATH: &str = "ISteamUser/ResolveVanityURL/v1/";
        text(vanity, "vanity URL name must not be empty or contain NUL")?;
        #[derive(Deserialize)]
        struct Resolved {
            steamid: SteamId,
        }
        let result: Result<Resolved> = self
            .0
            .call(
                Endpoint::get(PATH, Access::User),
                vec![param("vanityurl", vanity), param("url_type", kind.value())],
                &["response"],
            )
            .await;
        match result {
            Ok(response) if response.steamid.0 != 0 => Ok(Some(response.steamid)),
            Ok(_) => Err(Error::Api {
                endpoint: PATH,
                code: None,
            }),
            Err(Error::Api { code: Some(42), .. }) => Ok(None),
            Err(error) => Err(error),
        }
    }
}
pub(crate) fn validate_user_app(steamid: SteamId, appid: u32) -> Result<()> {
    nonzero(steamid.0, "steamid must not be zero")?;
    nonzero(appid.into(), "appid must not be zero")
}
fn validate_batch(len: usize) -> Result<()> {
    if !(1..=100).contains(&len) {
        return Err(Error::InvalidParameter("batch must contain 1..=100 IDs"));
    }
    Ok(())
}
fn validate_steamids(ids: &[SteamId]) -> Result<()> {
    validate_batch(ids.len())?;
    for id in ids {
        nonzero(id.0, "steamid must not be zero")?;
    }
    Ok(())
}
fn comma_list<T: std::fmt::Display>(values: &[T]) -> String {
    values
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(",")
}
