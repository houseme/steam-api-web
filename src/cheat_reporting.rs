// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! ICheatReportingService: explicit cheat reports, game bans, and VAC sessions.
//!
//! Publisher credentials and anti-cheat opt-in are required. Steam validates
//! permission for the app; possession of a publisher key alone proves neither
//! authorization nor a cheating incident. Each mutation is a separate explicit
//! call and is never automatically replayed. A report never issues a ban here.
//!
//! Valve must manually enable the Web API session methods. They are for games
//! that authenticate users independently of Steam; games using Steam
//! Authentication should use the documented native P2P or game-server flow.

use crate::{
    Error, Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::{nonzero, text},
    user::validate_user_app,
    user_auth::OpaqueResponse,
};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use std::{collections::BTreeMap, fmt};

/// Access to the seven documented ICheatReportingService methods.
#[derive(Debug, Clone, Copy)]
pub struct CheatReportingService<'a>(pub(crate) &'a SteamClient);

/// An explicit game-ban duration. The permanent zero sentinel must be selected
/// by name; it cannot be confused with a zero-second temporary suspension.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameBanDuration {
    /// An infinite game ban, encoded as duration zero.
    Permanent,
    /// A positive duration in seconds. Steam describes durations shorter than
    /// one year as suspensions that are not visible on the user's profile.
    Seconds(u32),
}

impl Serialize for GameBanDuration {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        match self {
            Self::Permanent => serializer.serialize_u32(0),
            Self::Seconds(0) => Err(serde::ser::Error::custom(
                "temporary duration must be positive",
            )),
            Self::Seconds(seconds) => serializer.serialize_u32(*seconds),
        }
    }
}

/// An incident report. Publisher backends must authenticate both parties before
/// forwarding peer accusations; Steam treats these reports as untrusted hearsay.
#[derive(Clone, Copy, Serialize)]
pub struct ReportPlayerCheatingRequest {
    /// Reported user's Steam ID.
    pub steamid: SteamId,
    /// Application receiving the incident report.
    pub appid: u32,
    /// Reporting user or game server, when available.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steamidreporter: Option<SteamId>,
    /// Developer-defined classification or evidence identifier.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appdata: Option<u64>,
    /// Whether the source was a heuristic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heuristic: Option<bool>,
    /// Whether the source was a detection.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detection: Option<bool>,
    /// Whether the source was a player report.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playerreport: Option<bool>,
    /// Suppress the returned report ID. Use only when no game ban will be based
    /// on this report; this client never substitutes or manufactures an ID.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub noreportid: Option<bool>,
    /// Developer-defined game mode; zero is generic.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gamemode: Option<u32>,
    /// Beginning of the interesting evidence interval, as Unix seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suspicionstarttime: Option<u32>,
    /// Developer-defined severity scale.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<u32>,
}

/// A separate game-ban request after an incident has been reviewed and confirmed.
#[derive(Clone, Copy, Serialize)]
pub struct RequestPlayerGameBanRequest<'a> {
    /// User whose confirmed incident is being acted on.
    pub steamid: SteamId,
    /// Application owning the game ban.
    pub appid: u32,
    /// Original report ID returned by ReportPlayerCheating.
    pub reportid: u64,
    /// Explanation of the confirmed cheating infraction.
    pub cheatdescription: &'a str,
    /// Explicit suspension duration or permanent-ban choice.
    pub duration: GameBanDuration,
    /// Apply Steam's default ban-delay rules.
    pub delayban: bool,
    /// Additional flags, currently documented as unused; transmitted unchanged.
    pub flags: u32,
}

/// One report/ban query, retaining the caller's time interval and report cursor.
#[derive(Clone, Copy, Serialize)]
pub struct CheatingReportsRequest {
    /// Application whose reports are being reviewed.
    pub appid: u32,
    /// Query end boundary, as Unix seconds.
    pub timeend: u32,
    /// Query beginning, as Unix seconds; must not exceed timeend.
    pub timebegin: u32,
    /// Minimum report ID. Zero starts the initial range; persist the last
    /// processed report boundary yourself after handling each response.
    pub reportidmin: u64,
    /// Include reports. If explicitly false, includebans must be explicitly true.
    /// Omitted together with includebans retains Steam's undocumented defaults.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub includereports: Option<bool>,
    /// Include ban requests. If explicitly false, includereports must be true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub includebans: Option<bool>,
    /// Restrict the query to this user, when specified.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steamid: Option<SteamId>,
}

/// A VAC status request. The optional session identifier must come from Steam's
/// StartSecureMultiplayerSession result, never from an unauthenticated claim.
#[derive(Clone, Copy, Serialize)]
pub struct VacStatusRequest {
    /// User whose status is requested.
    pub steamid: SteamId,
    /// Game the user is playing.
    pub appid: u32,
    /// Previously established Web API session identifier, if applicable.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_id: Option<u64>,
}

/// Parameters ending a previously established Web API multiplayer session.
#[derive(Clone, Copy, Serialize)]
pub struct EndSecureMultiplayerSessionRequest {
    /// User whose session is ending.
    pub steamid: SteamId,
    /// Application owning the session.
    pub appid: u32,
    /// Exact identifier returned by StartSecureMultiplayerSession.
    pub session_id: u64,
}

/// VAC's readiness and session result. A successful request is insufficient to
/// authorize matchmaking: success can be true while session_verified is false.
/// Later VAC timeout callbacks can still invalidate an initially verified result.
#[derive(Deserialize)]
pub struct VacStatusResponse {
    /// Whether VAC completed this check. False can mean VAC is not ready yet;
    /// it is preserved as a response state rather than retried automatically.
    pub success: bool,
    /// Session verification result; absence is not interpreted as verification.
    pub session_verified: Option<bool>,
    /// Other VAC fields, including any ban-status data. Inspect explicitly and
    /// apply the application's access policy; these fields have no inferred schema.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// The documented acknowledgement that a Web API session ended.
#[derive(Debug, Deserialize)]
pub struct EndSecureMultiplayerSessionResponse {
    /// Steam's explicit acknowledgement. A false marker is a Steam API error.
    pub success: bool,
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
    ReportPlayerCheatingRequest,
    RequestPlayerGameBanRequest<'_>,
    CheatingReportsRequest,
    VacStatusRequest,
    EndSecureMultiplayerSessionRequest,
    VacStatusResponse,
);

impl CheatReportingService<'_> {
    /// Submit only an incident report. It does not call the separate ban method.
    pub async fn report_player_cheating(
        &self,
        request: &ReportPlayerCheatingRequest,
    ) -> Result<OpaqueResponse> {
        validate_user_app(request.steamid, request.appid)?;
        if let Some(reporter) = request.steamidreporter {
            nonzero(reporter.raw(), "steamidreporter must not be zero")?;
        }
        self.service(
            Endpoint::post(
                "ICheatReportingService/ReportPlayerCheating/v1/",
                Access::Publisher,
            ),
            request,
        )
        .await
    }

    /// Request a reviewed game ban once, without inferring adjudication or app
    /// permission. There is no documented idempotency identifier for this write.
    pub async fn request_player_game_ban(
        &self,
        request: &RequestPlayerGameBanRequest<'_>,
    ) -> Result<OpaqueResponse> {
        validate_user_app(request.steamid, request.appid)?;
        nonzero(request.reportid, "reportid must not be zero")?;
        text(
            request.cheatdescription,
            "cheatdescription must not be empty or contain NUL",
        )?;
        if request.duration == GameBanDuration::Seconds(0) {
            return Err(Error::InvalidParameter(
                "temporary duration must be positive; select Permanent explicitly",
            ));
        }
        self.service(
            Endpoint::post(
                "ICheatReportingService/RequestPlayerGameBan/v1/",
                Access::Publisher,
            ),
            request,
        )
        .await
    }

    /// Remove a game ban determined to be a false positive. Never replayed.
    pub async fn remove_player_game_ban(
        &self,
        steamid: SteamId,
        appid: u32,
    ) -> Result<OpaqueResponse> {
        validate_user_app(steamid, appid)?;
        self.service(
            Endpoint::post(
                "ICheatReportingService/RemovePlayerGameBan/v1/",
                Access::Publisher,
            ),
            &Player { steamid, appid },
        )
        .await
    }

    /// Read one report range for independent review. No ban decision, cursor
    /// advancement, or partial-result recovery is synthesized by this method.
    pub async fn get_cheating_reports(
        &self,
        request: &CheatingReportsRequest,
    ) -> Result<OpaqueResponse> {
        nonzero(request.appid.into(), "appid must not be zero")?;
        if request.timebegin > request.timeend {
            return Err(Error::InvalidParameter("timebegin must not exceed timeend"));
        }
        if (request.includereports == Some(false) && request.includebans != Some(true))
            || (request.includebans == Some(false) && request.includereports != Some(true))
        {
            return Err(Error::InvalidParameter(
                "a false report or ban selector requires the other selector to be true",
            ));
        }
        if let Some(steamid) = request.steamid {
            nonzero(steamid.raw(), "steamid must not be zero")?;
        }
        self.service(
            Endpoint::get(
                "ICheatReportingService/GetCheatingReports/v1/",
                Access::Publisher,
            ),
            request,
        )
        .await
    }

    /// Read VAC readiness and session verification once. This POST is never
    /// automatically retried, including when VAC reports it is not ready yet.
    pub async fn request_vac_status_for_user(
        &self,
        request: &VacStatusRequest,
    ) -> Result<VacStatusResponse> {
        validate_user_app(request.steamid, request.appid)?;
        if let Some(session_id) = request.session_id {
            nonzero(session_id, "session_id must not be zero")?;
        }
        self.0
            .call(
                Endpoint::post(
                    "ICheatReportingService/RequestVacStatusForUser/v1/",
                    Access::Publisher,
                )
                .vac_status(),
                Params::json(request)?,
                &["response"],
            )
            .await
    }

    /// Start a Valve-enabled Web API session once. For games with Steam
    /// Authentication, use Steam's native authentication session flow instead.
    pub async fn start_secure_multiplayer_session(
        &self,
        steamid: SteamId,
        appid: u32,
    ) -> Result<OpaqueResponse> {
        validate_user_app(steamid, appid)?;
        self.service(
            Endpoint::post(
                "ICheatReportingService/StartSecureMultiplayerSession/v1/",
                Access::Publisher,
            ),
            &Player { steamid, appid },
        )
        .await
    }

    /// End the exact previously established Web API session without replay.
    pub async fn end_secure_multiplayer_session(
        &self,
        request: &EndSecureMultiplayerSessionRequest,
    ) -> Result<EndSecureMultiplayerSessionResponse> {
        validate_user_app(request.steamid, request.appid)?;
        nonzero(request.session_id, "session_id must not be zero")?;
        self.0
            .call(
                Endpoint::post(
                    "ICheatReportingService/EndSecureMultiplayerSession/v1/",
                    Access::Publisher,
                ),
                Params::json(request)?,
                &["response"],
            )
            .await
    }

    async fn service(
        &self,
        endpoint: Endpoint,
        request: &impl Serialize,
    ) -> Result<OpaqueResponse> {
        self.0.call_opaque(endpoint, Params::json(request)?).await
    }
}

#[derive(Serialize)]
struct Player {
    steamid: SteamId,
    appid: u32,
}
