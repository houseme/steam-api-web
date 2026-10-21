// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Publisher game notifications and asynchronous game sessions.
//!
//! All seven [documented methods](https://partner.steamgames.com/doc/webapi/IGameNotificationsService)
//! use `input_json` and require a publisher key. These sessions are notification
//! records, not native Steam matchmaking lobbies. Message and user substructures
//! remain explicitly reviewed JSON because the reference supplies examples rather
//! than a complete nested schema. Results preserve opaque, redacted JSON.
//! Every mutation is sent once. Requesting notifications is always explicit.

use crate::{
    Error, Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::{nonzero, text},
    user_auth::OpaqueResponse,
};
use serde::Serialize;
use serde_json::Value;
use std::fmt;

/// Access to the seven documented publisher game-notification methods.
#[derive(Debug, Clone, Copy)]
pub struct GameNotificationsService<'a>(pub(crate) &'a SteamClient);

/// Initial state for a new asynchronous notification session.
/// Debug hides private localized messages and user state.
#[derive(Clone, Copy, Serialize)]
pub struct CreateSessionRequest<'a> {
    /// Application owning this notification session.
    pub appid: u32,
    /// Application-defined context, including zero when meaningful to the game.
    pub context: u64,
    /// Localization message object reviewed against the game's Steam contract.
    pub title: &'a Value,
    /// Initial user-state objects. Unknown nested fields are preserved unchanged.
    pub users: &'a [Value],
    /// Optional user on whose behalf to create the session. Steam enforces the
    /// documented membership and friendship requirements when this is supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steamid: Option<SteamId>,
}

/// Replacement state for an asynchronous notification session.
///
/// Valve labels `title` and `users` optional but also warns that the complete
/// session state is replaced and omitted users are removed. Supply complete user
/// state when updating membership; this binding preserves omission and empty
/// arrays without deciding what the server will retain. Debug is redacted.
#[derive(Clone, Copy, Serialize)]
pub struct UpdateSessionRequest<'a> {
    /// Session ID returned by Steam.
    pub sessionid: u64,
    /// Application owning the session.
    pub appid: u32,
    /// Optional replacement localization message object.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<&'a Value>,
    /// Optional complete replacement list of user-state objects.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub users: Option<&'a [Value]>,
    /// Optional user on whose behalf to perform the update.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steamid: Option<SteamId>,
}

/// Filters and optional localization for listing one user's sessions.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct EnumerateSessionsRequest<'a> {
    /// User whose notification sessions are being read.
    pub steamid: SteamId,
    /// Optional application filter. Valve's prose permits omission to list all
    /// sessions, even though its parameter table marks this field required.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub appid: Option<u32>,
    /// Include messages for every user; omitted retains Steam's default of false.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_all_user_messages: Option<bool>,
    /// Include the authenticated user's message.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_auth_user_message: Option<bool>,
    /// Optional Steam API language name.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<&'a str>,
}

/// One explicitly requested session in a details batch.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SessionDetailsQuery {
    /// Nonzero session ID, serialized as an exact JSON uint64.
    pub sessionid: u64,
    /// Whether to include messages for every session user.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_all_user_messages: Option<bool>,
}

/// A batch of session-details queries for one application.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct SessionDetailsRequest<'a> {
    /// At least one session query; the service accepts a JSON message array.
    pub sessions: &'a [SessionDetailsQuery],
    /// Application owning the requested sessions.
    pub appid: u32,
    /// Steam API language for localized text.
    pub language: &'a str,
}

/// One session deletion, optionally performed on a user's behalf.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct DeleteSessionRequest {
    /// Session to delete, removing its users' notifications.
    pub sessionid: u64,
    /// Application owning the session.
    pub appid: u32,
    /// Optional user; Steam checks that this user belongs to the session.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub steamid: Option<SteamId>,
}

impl GameNotificationsService<'_> {
    /// Create a new asynchronous session once. Store Steam's returned session ID
    /// explicitly; this method does not dispatch a separate opt-in request.
    pub async fn create_session(
        &self,
        request: &CreateSessionRequest<'_>,
    ) -> Result<OpaqueResponse> {
        nonzero(request.appid.into(), "appid must not be zero")?;
        validate_message(request.title)?;
        validate_users(request.users)?;
        validate_actor(request.steamid)?;
        self.0
            .call_opaque(
                Endpoint::post(
                    "IGameNotificationsService/CreateSession/v1/",
                    Access::Publisher,
                ),
                Params::json(request)?,
            )
            .await
    }

    /// Replace session state once; supply the complete intended user state.
    /// Omitted fields remain omitted, and an explicit empty user array survives.
    pub async fn update_session(
        &self,
        request: &UpdateSessionRequest<'_>,
    ) -> Result<OpaqueResponse> {
        validate_session(request.appid, request.sessionid)?;
        if let Some(title) = request.title {
            validate_message(title)?;
        }
        if let Some(users) = request.users {
            validate_users(users)?;
        }
        validate_actor(request.steamid)?;
        self.0
            .call_opaque(
                Endpoint::post(
                    "IGameNotificationsService/UpdateSession/v1/",
                    Access::Publisher,
                ),
                Params::json(request)?,
            )
            .await
    }

    /// Read a user's notification sessions with optional localization (GET v1).
    pub async fn enumerate_sessions_for_app(
        &self,
        request: &EnumerateSessionsRequest<'_>,
    ) -> Result<OpaqueResponse> {
        nonzero(request.steamid.raw(), "steamid must not be zero")?;
        if let Some(appid) = request.appid {
            nonzero(appid.into(), "appid must not be zero when supplied")?;
        }
        if let Some(language) = request.language {
            text(language, "language must not be empty or contain NUL")?;
        }
        self.0
            .call_opaque(
                Endpoint::get(
                    "IGameNotificationsService/EnumerateSessionsForApp/v1/",
                    Access::Publisher,
                ),
                Params::json(request)?,
            )
            .await
    }

    /// Read a nonempty batch of session IDs (GET v1), retaining full uint64 IDs.
    /// Steam's unspecified response envelope remains opaque.
    pub async fn get_session_details_for_app(
        &self,
        request: &SessionDetailsRequest<'_>,
    ) -> Result<OpaqueResponse> {
        nonzero(request.appid.into(), "appid must not be zero")?;
        text(
            request.language,
            "language must not be empty or contain NUL",
        )?;
        if request.sessions.is_empty() || request.sessions.iter().any(|query| query.sessionid == 0)
        {
            return Err(Error::InvalidParameter(
                "sessions must contain nonzero session IDs",
            ));
        }
        self.0
            .call_opaque(
                Endpoint::get(
                    "IGameNotificationsService/GetSessionDetailsForApp/v1/",
                    Access::Publisher,
                ),
                Params::json(request)?,
            )
            .await
    }

    /// Explicitly request notification opt-in for a signed-in user (POST v1).
    /// Call at most once per game launch, as required by Valve. This method is
    /// never replayed automatically and does not infer consent or delivery.
    pub async fn request_notifications(
        &self,
        steamid: SteamId,
        appid: u32,
    ) -> Result<OpaqueResponse> {
        nonzero(steamid.raw(), "steamid must not be zero")?;
        nonzero(appid.into(), "appid must not be zero")?;
        #[derive(Serialize)]
        struct Request {
            steamid: SteamId,
            appid: u32,
        }
        self.0
            .call_opaque(
                Endpoint::post(
                    "IGameNotificationsService/RequestNotifications/v1/",
                    Access::Publisher,
                ),
                Params::json(&Request { steamid, appid })?,
            )
            .await
    }

    /// Delete one active asynchronous session once, removing its notifications.
    pub async fn delete_session(&self, request: &DeleteSessionRequest) -> Result<OpaqueResponse> {
        validate_session(request.appid, request.sessionid)?;
        validate_actor(request.steamid)?;
        self.0
            .call_opaque(
                Endpoint::post(
                    "IGameNotificationsService/DeleteSession/v1/",
                    Access::Publisher,
                ),
                Params::json(request)?,
            )
            .await
    }

    /// Invoke DeleteSessionBatch once using its documented scalar `sessionid`.
    /// Despite the method's name, Valve supplies no multi-ID array or count
    /// contract. This binding does not invent one or fan out additional calls.
    pub async fn delete_session_batch(&self, sessionid: u64, appid: u32) -> Result<OpaqueResponse> {
        validate_session(appid, sessionid)?;
        #[derive(Serialize)]
        struct Request {
            sessionid: u64,
            appid: u32,
        }
        self.0
            .call_opaque(
                Endpoint::post(
                    "IGameNotificationsService/DeleteSessionBatch/v1/",
                    Access::Publisher,
                ),
                Params::json(&Request { sessionid, appid })?,
            )
            .await
    }
}

fn validate_session(appid: u32, sessionid: u64) -> Result<()> {
    nonzero(appid.into(), "appid must not be zero")?;
    nonzero(sessionid, "sessionid must not be zero")
}

fn validate_actor(steamid: Option<SteamId>) -> Result<()> {
    if let Some(steamid) = steamid {
        nonzero(steamid.raw(), "steamid must not be zero when supplied")?;
    }
    Ok(())
}

fn validate_message(message: &Value) -> Result<()> {
    if message.is_object() {
        Ok(())
    } else {
        Err(Error::InvalidParameter(
            "localization message must be a reviewed JSON object",
        ))
    }
}

fn validate_users(users: &[Value]) -> Result<()> {
    for user in users {
        if !user.is_object() {
            return Err(Error::InvalidParameter(
                "users must contain reviewed JSON objects",
            ));
        }
        // The example documents steamid, but not every possible user field.
        // Validate that ID when present without inventing other required fields.
        if let Some(id) = user.get("steamid") {
            let valid = id.as_u64().or_else(|| id.as_str()?.parse::<u64>().ok());
            if valid.is_none_or(|id| id == 0) {
                return Err(Error::InvalidParameter(
                    "user steamid must be a nonzero uint64",
                ));
            }
        }
    }
    Ok(())
}

impl fmt::Debug for CreateSessionRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CreateSessionRequest([REDACTED])")
    }
}

impl fmt::Debug for UpdateSessionRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("UpdateSessionRequest([REDACTED])")
    }
}
