// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! ISteamCommunity: explicit publisher reports of abuse on a community hub.
//!
//! Requests use ordinary form fields, with Valve's case-sensitive names. The
//! official reference does not publish a complete response schema; results remain
//! opaque and Debug output excludes report narratives and identities.

use crate::{
    Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::{nonzero, text},
    user_auth::OpaqueResponse,
};
use std::fmt;

/// Access to the documented ISteamCommunity report method.
#[derive(Debug, Clone, Copy)]
pub struct Community<'a>(pub(crate) &'a SteamClient);

/// An authenticated user's explicit abuse report. Numeric report/content type
/// codes remain open because the reference does not enumerate their values.
#[derive(Clone, Copy)]
pub struct ReportAbuseRequest<'a> {
    /// Steam ID of the user making the report.
    pub steamid_actor: SteamId,
    /// Steam ID of the accused entity; not necessarily an individual account.
    pub steamid_target: SteamId,
    /// Application whose ownership Steam should check.
    pub appid: u32,
    /// EAbuseReportType code, transmitted without inventing an enum mapping.
    pub abuse_type: u32,
    /// ECommunityContentType code, transmitted unchanged.
    pub content_type: u32,
    /// User's narrative.
    pub description: &'a str,
    /// Related record ID, with meaning determined by content_type.
    pub gid: Option<u64>,
}

impl fmt::Debug for ReportAbuseRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("ReportAbuseRequest([REDACTED])")
    }
}

impl Community<'_> {
    /// Submit one abuse report with publisher credentials. No automatic replay,
    /// adjudication, or subsequent moderation action is performed by this call.
    pub async fn report_abuse(&self, request: &ReportAbuseRequest<'_>) -> Result<OpaqueResponse> {
        nonzero(request.steamid_actor.raw(), "steamidActor must not be zero")?;
        nonzero(
            request.steamid_target.raw(),
            "steamidTarget must not be zero",
        )?;
        nonzero(request.appid.into(), "appid must not be zero")?;
        text(
            request.description,
            "description must not be empty or contain NUL",
        )?;
        let mut params = Params::new()
            .with("steamidActor", request.steamid_actor)
            .with("steamidTarget", request.steamid_target)
            .with("appid", request.appid)
            .with("abuseType", request.abuse_type)
            .with("contentType", request.content_type)
            .with("description", request.description);
        if let Some(gid) = request.gid {
            params.push("gid", gid);
        }
        self.0
            .call_opaque(
                Endpoint::post("ISteamCommunity/ReportAbuse/v1/", Access::Publisher),
                params,
            )
            .await
    }
}
