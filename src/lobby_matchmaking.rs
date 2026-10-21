// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! The three documented `ILobbyMatchmakingService` Web API methods.
//!
//! Requests use `input_json`. CreateLobby follows the reference's user-key
//! parameter; the other two methods follow its explicit publisher-key notes.
//! No native SDK lobby lifecycle or notification-session state is inferred.
//! Incomplete response schemas retain their original JSON with redacted Debug.

use crate::{
    Error, Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::{nonzero, text},
    user_auth::OpaqueResponse,
};
use serde::Serialize;
use std::fmt;

/// Access to Web API lobby creation, member removal, and data reads.
#[derive(Debug, Clone, Copy)]
pub struct LobbyMatchmakingService<'a>(pub(crate) &'a SteamClient);

/// One lobby metadata entry using the reference's documented field names.
/// Debug is redacted because values can contain private game data.
#[derive(Clone, Copy, Serialize)]
pub struct LobbyMetadata<'a> {
    /// Metadata key.
    pub key_name: &'a str,
    /// Metadata value; an explicit empty string is preserved.
    pub key_value: &'a str,
}

/// One create-or-replace lobby request. Mutations are never replayed.
///
/// For type 4, reusing the same application and lobby name finds the persistent
/// unique lobby and replaces its entire allowed-members list. Previously allowed
/// users absent from this request are removed. Debug is redacted.
#[derive(Clone, Copy, Serialize)]
pub struct CreateLobbyRequest<'a> {
    /// Application owning the lobby.
    pub appid: u32,
    /// Positive signed 32-bit member capacity.
    pub max_members: i32,
    /// Documented `ELobbyType` numeric value; 4 is PrivateUnique. The key is
    /// `lobby_type` as in the table/prose; Valve's example spells it `lobbytype`.
    pub lobby_type: i32,
    /// Unique name required for type 4; omitted for an unnamed ephemeral lobby.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lobby_name: Option<&'a str>,
    /// Complete allowed-members list. Empty arrays remain explicit, permitting
    /// the caller to remove all reserved slots from an existing unique lobby.
    pub steamid_invited_members: &'a [SteamId],
    /// Optional documented metadata message array; empty values remain explicit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lobby_metadata: Option<&'a [LobbyMetadata<'a>]>,
}

impl LobbyMatchmakingService<'_> {
    /// Create or update the complete membership of a named lobby (POST v1).
    /// The user-key requirement follows Valve's parameter table; publisher keys
    /// are also accepted by the shared client. No native lobby calls are made.
    pub async fn create_lobby(&self, request: &CreateLobbyRequest<'_>) -> Result<OpaqueResponse> {
        nonzero(request.appid.into(), "appid must not be zero")?;
        if request.max_members <= 0 {
            return Err(Error::InvalidParameter("max_members must be positive"));
        }
        // Numeric values are retained for compatibility with future lobby types;
        // the published type field is int32 and the reference explicitly uses 4.
        if request.lobby_type < 0 {
            return Err(Error::InvalidParameter("lobby_type must not be negative"));
        }
        if request.lobby_type == 4 && request.lobby_name.is_none() {
            return Err(Error::InvalidParameter(
                "private unique lobbies require lobby_name",
            ));
        }
        if let Some(name) = request.lobby_name {
            text(name, "lobby_name must not be empty or contain NUL")?;
        }
        if request
            .steamid_invited_members
            .iter()
            .any(|id| id.raw() == 0)
        {
            return Err(Error::InvalidParameter(
                "invited Steam IDs must not be zero",
            ));
        }
        if let Some(metadata) = request.lobby_metadata {
            for entry in metadata {
                text(
                    entry.key_name,
                    "metadata key must not be empty or contain NUL",
                )?;
                if entry.key_value.contains('\0') {
                    return Err(Error::InvalidParameter(
                        "metadata value must not contain NUL",
                    ));
                }
            }
        }
        self.0
            .call_opaque(
                Endpoint::post("ILobbyMatchmakingService/CreateLobby/v1/", Access::User),
                Params::json(request)?,
            )
            .await
    }

    /// Remove a user or their offline reserved slot (publisher POST v1), once.
    pub async fn remove_user_from_lobby(
        &self,
        appid: u32,
        steamid_to_remove: SteamId,
        steamid_lobby: SteamId,
    ) -> Result<OpaqueResponse> {
        validate_lobby(appid, steamid_lobby)?;
        nonzero(
            steamid_to_remove.raw(),
            "steamid_to_remove must not be zero",
        )?;
        #[derive(Serialize)]
        struct Request {
            appid: u32,
            steamid_to_remove: SteamId,
            steamid_lobby: SteamId,
        }
        self.0
            .call_opaque(
                Endpoint::post(
                    "ILobbyMatchmakingService/RemoveUserFromLobby/v1/",
                    Access::Publisher,
                ),
                Params::json(&Request {
                    appid,
                    steamid_to_remove,
                    steamid_lobby,
                })?,
            )
            .await
    }

    /// Read lobby metadata and members (publisher GET v1), preserving the opaque
    /// result without inferring presence or cross-service membership.
    pub async fn get_lobby_data(
        &self,
        appid: u32,
        steamid_lobby: SteamId,
    ) -> Result<OpaqueResponse> {
        validate_lobby(appid, steamid_lobby)?;
        #[derive(Serialize)]
        struct Request {
            appid: u32,
            steamid_lobby: SteamId,
        }
        self.0
            .call_opaque(
                Endpoint::get(
                    "ILobbyMatchmakingService/GetLobbyData/v1/",
                    Access::Publisher,
                ),
                Params::json(&Request {
                    appid,
                    steamid_lobby,
                })?,
            )
            .await
    }
}

fn validate_lobby(appid: u32, steamid_lobby: SteamId) -> Result<()> {
    nonzero(appid.into(), "appid must not be zero")?;
    nonzero(steamid_lobby.raw(), "steamid_lobby must not be zero")
}

impl fmt::Debug for LobbyMetadata<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("LobbyMetadata([REDACTED])")
    }
}

impl fmt::Debug for CreateLobbyRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CreateLobbyRequest([REDACTED])")
    }
}
