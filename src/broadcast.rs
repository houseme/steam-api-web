// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! IBroadcastService: explicit broadcast metadata submission from a trusted server.
//!
//! The frame payload is an opaque UTF-8 string. This module neither interprets
//! it as JSON/base64 nor starts a broadcast. Every POST is sent once; callers
//! must decide how to reconcile an ambiguous network outcome with Steam.

use crate::{
    Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::nonzero,
    user_auth::OpaqueResponse,
};
use serde::Serialize;
use std::fmt;

/// Access to Steam's documented broadcast metadata method.
#[derive(Debug, Clone, Copy)]
pub struct BroadcastService<'a>(pub(crate) &'a SteamClient);

/// One metadata frame for an existing broadcast. Debug hides the frame contents.
#[derive(Clone, Copy, Serialize)]
pub struct PostGameDataFrameRequest<'a> {
    /// Application supplying metadata for the broadcast.
    pub appid: u32,
    /// Broadcaster's full-width Steam ID, serialized without precision loss.
    pub steamid: SteamId,
    /// Existing broadcast identifier, preserving the complete uint64 range.
    pub broadcast_id: u64,
    /// Opaque frame string, transmitted unchanged, including empty strings.
    /// Steam's reference does not specify its internal schema or encoding.
    pub frame_data: &'a str,
}

impl fmt::Debug for PostGameDataFrameRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("PostGameDataFrameRequest([REDACTED])")
    }
}

impl BroadcastService<'_> {
    /// Submit one metadata frame as form-encoded input_json, without replaying
    /// failures or inspecting the undocumented successful response schema.
    /// Steam separately validates publisher permission for the app/broadcast.
    pub async fn post_game_data_frame(
        &self,
        request: &PostGameDataFrameRequest<'_>,
    ) -> Result<OpaqueResponse> {
        nonzero(request.appid.into(), "appid must not be zero")?;
        nonzero(request.steamid.raw(), "steamid must not be zero")?;
        nonzero(request.broadcast_id, "broadcast_id must not be zero")?;
        self.0
            .call_opaque(
                Endpoint::post("IBroadcastService/PostGameDataFrame/v1/", Access::Publisher),
                Params::json(request)?,
            )
            .await
    }
}
