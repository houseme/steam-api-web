// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! IPlayerService: games, playtime, Steam levels, and community badges.
//!
//! Service requests use `input_json`. The official reference does not specify
//! response schemas, so these methods retain the full JSON response, including
//! distinctions between hidden game details, empty results, and absent fields.

use crate::{
    Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::nonzero,
    user::validate_user_app,
    user_auth::OpaqueResponse,
};
use serde::Serialize;

/// Access to IPlayerService methods.
#[derive(Debug, Clone, Copy)]
pub struct PlayerService<'a>(pub(crate) &'a SteamClient);

/// Filters for a user's visible game library.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct OwnedGamesOptions<'a> {
    /// Request each game's name and icon in addition to its playtime.
    pub include_appinfo: bool,
    /// Include free games that the user has played.
    pub include_played_free_games: bool,
    /// Restrict the response to these nonzero app IDs; empty means no filter.
    #[serde(skip_serializing_if = "<[u32]>::is_empty")]
    pub appids_filter: &'a [u32],
}

impl PlayerService<'_> {
    /// Read recently played games. A count of zero requests all recent games.
    pub async fn get_recently_played_games(
        &self,
        steamid: impl Into<SteamId>,
        count: u32,
    ) -> Result<OpaqueResponse> {
        let steamid = steamid.into();
        nonzero(steamid.0, "steamid must not be zero")?;
        self.0
            .call_opaque(
                Endpoint::get("IPlayerService/GetRecentlyPlayedGames/v1/", Access::User),
                Params::json(&RecentGamesRequest { steamid, count })?,
            )
            .await
    }

    /// Read playtime for one app. Steam requires a key associated with the app;
    /// the client validates the key's presence and Steam validates its scope.
    pub async fn get_single_game_playtime(
        &self,
        steamid: impl Into<SteamId>,
        appid: u32,
    ) -> Result<OpaqueResponse> {
        let steamid = steamid.into();
        validate_user_app(steamid, appid)?;
        self.0
            .call_opaque(
                Endpoint::get("IPlayerService/GetSingleGamePlaytime/v1/", Access::User),
                Params::json(&PlaytimeRequest { steamid, appid })?,
            )
            .await
    }

    /// Read a visible owned-game library. Privacy may suppress the fields in a
    /// successful response; callers must not interpret missing fields as zero.
    pub async fn get_owned_games(
        &self,
        steamid: impl Into<SteamId>,
        options: OwnedGamesOptions<'_>,
    ) -> Result<OpaqueResponse> {
        let steamid = steamid.into();
        nonzero(steamid.0, "steamid must not be zero")?;
        for &appid in options.appids_filter {
            nonzero(appid.into(), "appids_filter must not contain zero")?;
        }
        #[derive(Serialize)]
        struct Request<'a> {
            steamid: SteamId,
            #[serde(flatten)]
            options: OwnedGamesOptions<'a>,
        }
        self.0
            .call_opaque(
                Endpoint::get("IPlayerService/GetOwnedGames/v1/", Access::User),
                Params::json(&Request { steamid, options })?,
            )
            .await
    }

    /// Read the user's Steam level.
    pub async fn get_steam_level(&self, steamid: impl Into<SteamId>) -> Result<OpaqueResponse> {
        let steamid = steamid.into();
        nonzero(steamid.0, "steamid must not be zero")?;
        self.0
            .call_opaque(
                Endpoint::get("IPlayerService/GetSteamLevel/v1/", Access::User),
                Params::json(&UserRequest { steamid })?,
            )
            .await
    }

    /// Read badges owned by the user.
    pub async fn get_badges(&self, steamid: impl Into<SteamId>) -> Result<OpaqueResponse> {
        let steamid = steamid.into();
        nonzero(steamid.0, "steamid must not be zero")?;
        self.0
            .call_opaque(
                Endpoint::get("IPlayerService/GetBadges/v1/", Access::User),
                Params::json(&UserRequest { steamid })?,
            )
            .await
    }

    /// Read quests and completion state for a community badge. The badge ID is
    /// signed on the wire; this method preserves the documented `int32` range.
    pub async fn get_community_badge_progress(
        &self,
        steamid: impl Into<SteamId>,
        badgeid: i32,
    ) -> Result<OpaqueResponse> {
        let steamid = steamid.into();
        nonzero(steamid.0, "steamid must not be zero")?;
        self.0
            .call_opaque(
                Endpoint::get("IPlayerService/GetCommunityBadgeProgress/v1/", Access::User),
                Params::json(&BadgeRequest { steamid, badgeid })?,
            )
            .await
    }
}

#[derive(Serialize)]
struct RecentGamesRequest {
    steamid: SteamId,
    count: u32,
}

#[derive(Serialize)]
struct PlaytimeRequest {
    steamid: SteamId,
    appid: u32,
}

#[derive(Serialize)]
struct UserRequest {
    steamid: SteamId,
}

#[derive(Serialize)]
struct BadgeRequest {
    steamid: SteamId,
    badgeid: i32,
}
