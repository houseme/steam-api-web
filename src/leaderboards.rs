// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Server-side leaderboard administration through `ISteamLeaderboards`.
//!
//! Every method requires a publisher API key. The
//! [official reference](https://partner.steamgames.com/doc/webapi/ISteamLeaderboards)
//! does not specify response schemas; results preserve the original JSON.

use crate::{
    Error, Result, SteamClient, SteamId,
    client::{Access, Endpoint, Params, param},
    types::{nonzero, text},
    user_auth::OpaqueResponse,
};

/// Access to all seven documented leaderboard administration methods.
#[derive(Debug, Clone, Copy)]
pub struct Leaderboards<'a>(pub(crate) &'a SteamClient);

/// Optional settings for finding or creating a leaderboard.
/// Omitted fields use Steam's defaults.
#[derive(Debug, Clone, Copy, Default)]
pub struct FindLeaderboardOptions<'a> {
    /// Steam sort method name; its documented default is `Ascending`.
    pub sortmethod: Option<&'a str>,
    /// Steam display type name; its documented default is `Numeric`.
    pub displaytype: Option<&'a str>,
    /// Create an absent leaderboard; Steam defaults to true.
    pub createifnotfound: Option<bool>,
    /// Restrict score writes to publisher Web API calls.
    pub onlytrustedwrites: Option<bool>,
    /// Restrict native client reads to friends.
    pub onlyfriendsreads: Option<bool>,
}

/// The documented entry selection modes for the Web API.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderboardDataRequest {
    /// Read an absolute global rank range.
    Global,
    /// Read signed rank offsets around a specified user.
    AroundUser,
    /// Read friends of a specified user.
    Friends,
}

impl LeaderboardDataRequest {
    fn wire_name(self) -> &'static str {
        match self {
            Self::Global => "RequestGlobal",
            Self::AroundUser => "RequestAroundUser",
            Self::Friends => "RequestFriends",
        }
    }
}

/// Parameters for a leaderboard entry query.
#[derive(Debug, Clone, Copy)]
pub struct LeaderboardEntriesRequest {
    /// Game application ID.
    pub appid: u32,
    /// Positive leaderboard ID. This read endpoint documents an `int32` field.
    pub leaderboardid: i32,
    /// Inclusive start rank or signed around-user offset.
    pub rangestart: i32,
    /// Inclusive end rank or signed around-user offset.
    pub rangeend: i32,
    /// Entry selection mode.
    pub datarequest: LeaderboardDataRequest,
    /// Required for friend and around-user queries; optional for global reads.
    pub steamid: Option<SteamId>,
}

/// How to apply a score update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LeaderboardScoreMethod {
    /// Preserve the existing score unless the new score is better.
    KeepBest,
    /// Replace the existing score unconditionally.
    ForceUpdate,
}

impl LeaderboardScoreMethod {
    fn wire_name(self) -> &'static str {
        match self {
            Self::KeepBest => "KeepBest",
            Self::ForceUpdate => "ForceUpdate",
        }
    }
}

/// One publisher score update, including optional raw binary details.
#[derive(Debug, Clone, Copy)]
pub struct LeaderboardScoreUpdate<'a> {
    /// Game application ID.
    pub appid: u32,
    /// Target leaderboard's unsigned ID.
    pub leaderboardid: u32,
    /// User receiving the score.
    pub steamid: SteamId,
    /// Signed score; negative values are preserved.
    pub score: i32,
    /// Keep the best score or force replacement.
    pub scoremethod: LeaderboardScoreMethod,
    /// Optional binary details, at most 256 bytes; no UTF-8 conversion is used.
    pub details: Option<&'a [u8]>,
}

impl Leaderboards<'_> {
    /// Delete a named leaderboard. Never automatically replayed.
    pub async fn delete_leaderboard(&self, appid: u32, name: &str) -> Result<OpaqueResponse> {
        validate_name(appid, name)?;
        self.0
            .call(
                Endpoint::auth(
                    "ISteamLeaderboards/DeleteLeaderboard/v1/",
                    Access::Publisher,
                    true,
                ),
                vec![param("appid", appid), param("name", name)],
                &[],
            )
            .await
    }

    /// Remove a user's score without automatic replay.
    pub async fn delete_leaderboard_score(
        &self,
        appid: u32,
        leaderboardid: u32,
        steamid: SteamId,
    ) -> Result<OpaqueResponse> {
        let mut params = leaderboard_params(appid, leaderboardid)?;
        nonzero(steamid.raw(), "steamid must not be zero")?;
        params.push(param("steamid", steamid));
        self.0
            .call(
                Endpoint::auth(
                    "ISteamLeaderboards/DeleteLeaderboardScore/v1/",
                    Access::Publisher,
                    true,
                ),
                params,
                &[],
            )
            .await
    }

    /// Find or create a leaderboard using v2. Even a lookup-only request is sent
    /// once because the method is a POST with conditional creation semantics.
    pub async fn find_or_create_leaderboard(
        &self,
        appid: u32,
        name: &str,
        options: FindLeaderboardOptions<'_>,
    ) -> Result<OpaqueResponse> {
        validate_name(appid, name)?;
        let mut params = vec![param("appid", appid), param("name", name)];
        if let Some(value) = options.sortmethod {
            text(value, "sortmethod must be nonempty and contain no NUL")?;
            params.push(param("sortmethod", value));
        }
        if let Some(value) = options.displaytype {
            text(value, "displaytype must be nonempty and contain no NUL")?;
            params.push(param("displaytype", value));
        }
        for (name, value) in [
            ("createifnotfound", options.createifnotfound),
            ("onlytrustedwrites", options.onlytrustedwrites),
            ("onlyfriendsreads", options.onlyfriendsreads),
        ] {
            if let Some(value) = value {
                params.push(param(name, value));
            }
        }
        self.0
            .call(
                Endpoint::auth(
                    "ISteamLeaderboards/FindOrCreateLeaderboard/v2/",
                    Access::Publisher,
                    true,
                ),
                params,
                &[],
            )
            .await
    }

    /// Read entries, retaining signed around-user offsets and exact Steam IDs.
    pub async fn get_leaderboard_entries(
        &self,
        request: LeaderboardEntriesRequest,
    ) -> Result<OpaqueResponse> {
        nonzero(request.appid.into(), "appid must not be zero")?;
        if request.leaderboardid <= 0 {
            return Err(Error::InvalidParameter("leaderboardid must be positive"));
        }
        if request.rangestart > request.rangeend {
            return Err(Error::InvalidParameter(
                "rangestart must not exceed rangeend",
            ));
        }
        if request.datarequest == LeaderboardDataRequest::Global && request.rangestart < 0 {
            return Err(Error::InvalidParameter(
                "global rangestart must not be negative",
            ));
        }
        if request.datarequest != LeaderboardDataRequest::Global && request.steamid.is_none() {
            return Err(Error::InvalidParameter(
                "steamid required for friend and around-user reads",
            ));
        }
        let mut params = vec![
            param("appid", request.appid),
            param("leaderboardid", request.leaderboardid),
            param("rangestart", request.rangestart),
            param("rangeend", request.rangeend),
            param("datarequest", request.datarequest.wire_name()),
        ];
        if let Some(steamid) = request.steamid {
            nonzero(steamid.raw(), "steamid must not be zero")?;
            params.push(param("steamid", steamid));
        }
        self.0
            .call(
                Endpoint::get(
                    "ISteamLeaderboards/GetLeaderboardEntries/v1/",
                    Access::Publisher,
                ),
                params,
                &[],
            )
            .await
    }

    /// List a game's leaderboards through the latest documented v2 method.
    pub async fn get_leaderboards_for_game(&self, appid: u32) -> Result<OpaqueResponse> {
        nonzero(appid.into(), "appid must not be zero")?;
        self.0
            .call(
                Endpoint::get(
                    "ISteamLeaderboards/GetLeaderboardsForGame/v2/",
                    Access::Publisher,
                ),
                vec![param("appid", appid)],
                &[],
            )
            .await
    }

    /// Clear a leaderboard without automatic replay.
    pub async fn reset_leaderboard(
        &self,
        appid: u32,
        leaderboardid: u32,
    ) -> Result<OpaqueResponse> {
        let params = leaderboard_params(appid, leaderboardid)?;
        self.0
            .call(
                Endpoint::auth(
                    "ISteamLeaderboards/ResetLeaderboard/v1/",
                    Access::Publisher,
                    true,
                ),
                params,
                &[],
            )
            .await
    }

    /// Set a score and up to 256 raw detail bytes without automatic replay.
    pub async fn set_leaderboard_score(
        &self,
        update: LeaderboardScoreUpdate<'_>,
    ) -> Result<OpaqueResponse> {
        let mut params = leaderboard_params(update.appid, update.leaderboardid)?;
        nonzero(update.steamid.raw(), "steamid must not be zero")?;
        params.extend([
            param("steamid", update.steamid),
            param("score", update.score),
            param("scoremethod", update.scoremethod.wire_name()),
        ]);
        if let Some(details) = update.details {
            if details.len() > 256 {
                return Err(Error::InvalidParameter("details must not exceed 256 bytes"));
            }
            params.push(("details".to_owned(), details.to_vec()));
        }
        self.0
            .call(
                Endpoint::auth(
                    "ISteamLeaderboards/SetLeaderboardScore/v1/",
                    Access::Publisher,
                    true,
                ),
                params,
                &[],
            )
            .await
    }
}

fn validate_name(appid: u32, name: &str) -> Result<()> {
    nonzero(appid.into(), "appid must not be zero")?;
    text(name, "leaderboard name must be nonempty and contain no NUL")
}

fn leaderboard_params(appid: u32, leaderboardid: u32) -> Result<Params> {
    nonzero(appid.into(), "appid must not be zero")?;
    nonzero(leaderboardid.into(), "leaderboardid must not be zero")?;
    Ok(vec![
        param("appid", appid),
        param("leaderboardid", leaderboardid),
    ])
}
