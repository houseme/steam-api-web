// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! ISiteLicenseService: PC Cafe site activity and license-specific playtime.
//!
//! Both methods require publisher credentials. Method pages show the public
//! host, while the Web API overview recommends Partner for secure publishers.
//! Routing follows the client's explicit host configuration; select
//! [`crate::Host::Public`] to use the method-page host. No host fallback occurs.
//!
//! The reference describes the response fields without a complete JSON example.
//! These models use the standard `response` envelope, pending credentialed wire
//! validation. Unknown fields and license names are retained. Game-name IDs are
//! response-local dictionary references and must never be treated as app IDs.

use crate::{
    Error, Result, SteamClient,
    request::{Access, Endpoint, Params},
    types::{compare_utc_times, u64_string},
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;
use std::{cmp::Ordering, collections::BTreeMap, fmt};

/// Access to the two documented PC Cafe site methods.
#[derive(Debug, Clone, Copy)]
pub struct SiteLicenseService<'a>(pub(crate) &'a SteamClient);

/// A report of sessions starting within `[start_time, end_time)`.
#[derive(Clone, Copy, Serialize)]
pub struct TotalPlaytimeRequest<'a> {
    /// Inclusive start boundary in RFC 3339 UTC form, including optional fractions.
    pub start_time: &'a str,
    /// Exclusive end boundary in RFC 3339 UTC form. It must not precede start_time.
    pub end_time: &'a str,
    /// Site filter. Omitted and zero both request every authorized site.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub siteid: Option<u64>,
}

/// A game-name entry whose ID is local to this response's dictionary.
#[derive(Deserialize)]
pub struct SiteGameName {
    /// Local dictionary key referenced by a site's game records; not an app ID.
    pub id: u32,
    /// Display name supplied by Steam.
    pub name: String,
    /// Unrecognized fields retained for forward compatibility.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// The license category Steam returned, including future categories verbatim.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum SiteLicenseType {
    /// The player owns a personal license.
    User,
    /// A paid commercial license assigned to the site.
    Commercial,
    /// A free commercial license assigned to the site.
    Free,
    /// A category introduced by Steam or otherwise undocumented.
    Other(String),
}

impl SiteLicenseType {
    /// Inspect the exact wire spelling without collapsing unknown categories.
    pub fn as_str(&self) -> &str {
        match self {
            Self::User => "user",
            Self::Commercial => "commercial",
            Self::Free => "free",
            Self::Other(value) => value,
        }
    }
}

impl<'de> Deserialize<'de> for SiteLicenseType {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> std::result::Result<Self, D::Error> {
        Ok(match String::deserialize(deserializer)? {
            value if value == "user" => Self::User,
            value if value == "commercial" => Self::Commercial,
            value if value == "free" => Self::Free,
            value => Self::Other(value),
        })
    }
}

/// A current game session on one PC Cafe computer.
#[derive(Deserialize)]
pub struct SiteGameSession {
    /// Reference into the response's game_names dictionary; not an app ID.
    pub id: u32,
    /// Steam's original session start timestamp in RFC 3339 UTC form.
    pub start_time: String,
    /// Personal, commercial, free, or an unknown future category.
    pub license_type: SiteLicenseType,
    /// Unrecognized fields retained for forward compatibility.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// A PC Cafe computer and its current game sessions. Debug is redacted.
#[derive(Deserialize)]
pub struct SiteClient {
    /// Computer IP address. Treat this value as private operational information.
    pub ip_address: String,
    /// Computer hostname. Treat this value as private operational information.
    pub hostname: String,
    /// Active game sessions on this computer.
    pub games: Vec<SiteGameSession>,
    /// Unrecognized fields retained for forward compatibility.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// One site's currently connected computers. Debug is redacted.
#[derive(Deserialize)]
pub struct CurrentSiteConnections {
    /// Full-width site identifier, accepting decimal strings and JSON integers.
    #[serde(deserialize_with = "u64_string")]
    pub siteid: u64,
    /// Operator-assigned site name.
    pub site_name: String,
    /// Active computers at the site.
    pub clients: Vec<SiteClient>,
    /// Unrecognized fields retained for forward compatibility.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Current connections and their response-local game-name dictionary.
#[derive(Deserialize)]
pub struct CurrentClientConnectionsResponse {
    /// Sites visible to the authenticated publisher.
    pub sites: Vec<CurrentSiteConnections>,
    /// Dictionary mapping game-record IDs to names; IDs are not Steam app IDs.
    pub game_names: Vec<SiteGameName>,
    /// Unrecognized fields retained for forward compatibility.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Playtime for one response-local game ID and one license category.
#[derive(Deserialize)]
pub struct SiteGamePlaytime {
    /// Reference into game_names; not an app ID.
    pub id: u32,
    /// License category, preserving unknown future names.
    pub license_type: SiteLicenseType,
    /// Steam's uint32 playtime aggregate, in seconds, for the requested interval.
    pub playtime_seconds: u32,
    /// Unrecognized fields retained for forward compatibility.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// One site's aggregate game playtime, split by license category.
#[derive(Deserialize)]
pub struct SitePlaytime {
    /// Full-width site identifier, accepting decimal strings and JSON integers.
    #[serde(deserialize_with = "u64_string")]
    pub siteid: u64,
    /// Operator-assigned site name.
    pub site_name: String,
    /// One entry for each game and license category returned by Steam.
    pub games: Vec<SiteGamePlaytime>,
    /// Unrecognized fields retained for forward compatibility.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Aggregate playtime and the matching response-local game-name dictionary.
#[derive(Deserialize)]
pub struct TotalPlaytimeResponse {
    /// Sites visible to the authenticated publisher.
    pub sites: Vec<SitePlaytime>,
    /// Dictionary mapping game-record IDs to names; IDs are not Steam app IDs.
    pub game_names: Vec<SiteGameName>,
    /// Unrecognized fields retained for forward compatibility.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

macro_rules! redacted_debug {
    ($($name:ty),+ $(,)?) => {$(
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "([REDACTED])"))
            }
        }
    )+};
}

redacted_debug!(
    TotalPlaytimeRequest<'_>,
    SiteGameName,
    SiteGameSession,
    SiteClient,
    CurrentSiteConnections,
    CurrentClientConnectionsResponse,
    SiteGamePlaytime,
    SitePlaytime,
    TotalPlaytimeResponse,
);

impl SiteLicenseService<'_> {
    /// Read current activity. Omitted and zero site filters both mean all sites;
    /// this method never changes host or broadens a supplied nonzero filter.
    pub async fn get_current_client_connections(
        &self,
        siteid: Option<u64>,
    ) -> Result<CurrentClientConnectionsResponse> {
        #[derive(Serialize)]
        struct Request {
            #[serde(skip_serializing_if = "Option::is_none")]
            siteid: Option<u64>,
        }
        self.0
            .call(
                Endpoint::get(
                    "ISiteLicenseService/GetCurrentClientConnections/v1/",
                    Access::Publisher,
                ),
                Params::json(&Request { siteid })?,
                &["response"],
            )
            .await
    }

    /// Read sessions that started on/after start_time and before end_time.
    /// Boundaries, fractional precision, and the caller's site filter are sent
    /// unchanged. Equal boundaries describe an empty interval; no polling or
    /// report chunking is performed automatically.
    pub async fn get_total_playtime(
        &self,
        request: &TotalPlaytimeRequest<'_>,
    ) -> Result<TotalPlaytimeResponse> {
        match compare_utc_times(request.start_time, request.end_time) {
            Some(Ordering::Greater) => {
                return Err(Error::InvalidParameter(
                    "start_time must not exceed end_time",
                ));
            }
            None => {
                return Err(Error::InvalidParameter(
                    "start_time and end_time must be RFC 3339 UTC timestamps",
                ));
            }
            _ => {}
        }
        self.0
            .call(
                Endpoint::get(
                    "ISiteLicenseService/GetTotalPlaytime/v1/",
                    Access::Publisher,
                ),
                Params::json(request)?,
                &["response"],
            )
            .await
    }
}
