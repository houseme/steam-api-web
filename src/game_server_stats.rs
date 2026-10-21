// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! ISteamGameServerStats: publisher access to historical game server player stats.

use crate::{
    Error, Result, SteamClient,
    request::{Access, Endpoint, Params},
    types::nonzero,
    user_auth::OpaqueResponse,
};

/// Access to the documented ISteamGameServerStats Web method.
#[derive(Debug, Clone, Copy)]
pub struct GameServerStats<'a>(pub(crate) &'a SteamClient);

/// A game server statistics query with times in Seattle's local time zone.
///
/// Times must use `YYYY-MM-DD HH:MM:SS`. They are validated as calendar values
/// and transmitted unchanged; this client does not interpret local daylight
/// saving transitions or convert UTC timestamps.
#[derive(Debug, Clone, Copy)]
pub struct GameServerPlayerStatsRequest<'a> {
    /// Game ID; an unmodified game's application ID can be used here.
    pub gameid: u64,
    /// Application ID.
    pub appid: u32,
    /// First date and time in Seattle local time.
    pub rangestart: &'a str,
    /// Last date and time in Seattle local time; must not precede `rangestart`.
    pub rangeend: &'a str,
    /// Optional maximum result count, between one and 1000 inclusive.
    pub maxresults: Option<u32>,
}

impl GameServerStats<'_> {
    /// Read historical player stats (publisher GET v1).
    /// Valve does not document the response schema, so its JSON remains opaque.
    pub async fn get_game_server_player_stats_for_game(
        &self,
        request: &GameServerPlayerStatsRequest<'_>,
    ) -> Result<OpaqueResponse> {
        nonzero(request.gameid, "gameid must not be zero")?;
        nonzero(request.appid.into(), "appid must not be zero")?;
        if !valid_local_time(request.rangestart) || !valid_local_time(request.rangeend) {
            return Err(Error::InvalidParameter(
                "range timestamps must be valid Seattle local YYYY-MM-DD HH:MM:SS values",
            ));
        }
        if request.rangestart > request.rangeend {
            return Err(Error::InvalidParameter(
                "rangestart must not exceed rangeend",
            ));
        }
        if request
            .maxresults
            .is_some_and(|count| !(1..=1000).contains(&count))
        {
            return Err(Error::InvalidParameter("maxresults must be in 1..=1000"));
        }
        let mut params = Params::new()
            .with("gameid", request.gameid)
            .with("appid", request.appid)
            .with("rangestart", request.rangestart)
            .with("rangeend", request.rangeend);
        if let Some(maxresults) = request.maxresults {
            params.push("maxresults", maxresults);
        }
        self.0
            .call_opaque(
                Endpoint::get(
                    "ISteamGameServerStats/GetGameServerPlayerStatsForGame/v1/",
                    Access::Publisher,
                ),
                params,
            )
            .await
    }
}

fn valid_local_time(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 19
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b' '
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes
            .iter()
            .enumerate()
            .any(|(index, byte)| ![4, 7, 10, 13, 16].contains(&index) && !byte.is_ascii_digit())
    {
        return false;
    }
    let number = |start: usize, end: usize| {
        bytes[start..end]
            .iter()
            .fold(0_u32, |value, byte| value * 10 + u32::from(byte - b'0'))
    };
    let year = number(0, 4);
    let month = number(5, 7);
    let day = number(8, 10);
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    year != 0
        && (1..=days).contains(&day)
        && number(11, 13) <= 23
        && number(14, 16) <= 59
        && number(17, 19) <= 59
}
