// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Persistent game server accounts through `IGameServersService`.
//!
//! Every method uses `input_json` and requires a user or publisher API key.
//! The [official reference](https://partner.steamgames.com/doc/webapi/IGameServersService)
//! does not define response schemas, so results retain the original JSON with
//! redacted Debug output. Account responses may contain secret login tokens.

use crate::{
    Error, Result, SteamClient, SteamId,
    client::{Access, Endpoint, param},
    types::{nonzero, text},
    user_auth::OpaqueResponse,
};
use serde_json::{Value, json};
use std::{
    fmt,
    net::{IpAddr, SocketAddr},
};
use zeroize::Zeroizing;

/// Access to the nine documented game server account methods.
#[derive(Debug, Clone, Copy)]
pub struct GameServers<'a>(pub(crate) &'a SteamClient);

/// A secret persistent game server login token with redacted Debug output.
/// Owned text is zeroized when dropped.
pub struct GameServerLoginToken(Zeroizing<String>);

impl GameServerLoginToken {
    /// Validate and own a login token without imposing an undocumented format.
    pub fn new(value: impl Into<String>) -> Result<Self> {
        let value = Zeroizing::new(value.into());
        text(&value, "login_token must be nonempty and contain no NUL")?;
        Ok(Self(value))
    }

    /// Explicitly expose the token for transmission. Never log this value.
    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for GameServerLoginToken {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GameServerLoginToken([REDACTED])")
    }
}

impl GameServers<'_> {
    /// Get the current account's server list and secret login tokens.
    pub async fn get_account_list(&self) -> Result<OpaqueResponse> {
        self.call(
            Endpoint::get("IGameServersService/GetAccountList/v1/", Access::User),
            json!({}),
        )
        .await
    }

    /// Create a persistent account. This mutation is never automatically retried.
    /// An empty memo is permitted.
    pub async fn create_account(&self, appid: u32, memo: &str) -> Result<OpaqueResponse> {
        nonzero(appid.into(), "appid must not be zero")?;
        validate_memo(memo)?;
        self.call(
            Endpoint::auth("IGameServersService/CreateAccount/v1/", Access::User, true),
            json!({"appid": appid, "memo": memo}),
        )
        .await
    }

    /// Change or clear an account memo without automatic replay.
    pub async fn set_memo(&self, steamid: SteamId, memo: &str) -> Result<OpaqueResponse> {
        nonzero(steamid.raw(), "steamid must not be zero")?;
        validate_memo(memo)?;
        self.call(
            Endpoint::auth("IGameServersService/SetMemo/v1/", Access::User, true),
            json!({"steamid": steamid, "memo": memo}),
        )
        .await
    }

    /// Replace an account's login token. Never automatically replayed because a
    /// second successful call could invalidate the token from the first call.
    pub async fn reset_login_token(&self, steamid: SteamId) -> Result<OpaqueResponse> {
        nonzero(steamid.raw(), "steamid must not be zero")?;
        self.call(
            Endpoint::auth(
                "IGameServersService/ResetLoginToken/v1/",
                Access::User,
                true,
            ),
            json!({"steamid": steamid}),
        )
        .await
    }

    /// Delete a persistent account without automatic replay.
    pub async fn delete_account(&self, steamid: SteamId) -> Result<OpaqueResponse> {
        nonzero(steamid.raw(), "steamid must not be zero")?;
        self.call(
            Endpoint::auth("IGameServersService/DeleteAccount/v1/", Access::User, true),
            json!({"steamid": steamid}),
        )
        .await
    }

    /// Read an account's public information using an authenticated request.
    pub async fn get_account_public_info(&self, steamid: SteamId) -> Result<OpaqueResponse> {
        nonzero(steamid.raw(), "steamid must not be zero")?;
        self.call(
            Endpoint::get("IGameServersService/GetAccountPublicInfo/v1/", Access::User),
            json!({"steamid": steamid}),
        )
        .await
    }

    /// Query the status of a login token owned by the API key's account.
    pub async fn query_login_token(
        &self,
        login_token: &GameServerLoginToken,
    ) -> Result<OpaqueResponse> {
        self.call(
            Endpoint::get("IGameServersService/QueryLoginToken/v1/", Access::User),
            json!({"login_token": login_token.expose_secret()}),
        )
        .await
    }

    /// Look up server IDs by IP addresses, optionally including a port.
    /// Both IPv4 and IPv6 literals are accepted; hostnames are rejected.
    pub async fn get_server_steam_ids_by_ip(&self, server_ips: &[&str]) -> Result<OpaqueResponse> {
        if server_ips.is_empty() {
            return Err(Error::InvalidParameter("server_ips must not be empty"));
        }
        if server_ips
            .iter()
            .any(|ip| ip.parse::<IpAddr>().is_err() && ip.parse::<SocketAddr>().is_err())
        {
            return Err(Error::InvalidParameter(
                "server_ips must contain IP literals with optional ports",
            ));
        }
        self.call(
            Endpoint::get(
                "IGameServersService/GetServerSteamIDsByIP/v1/",
                Access::User,
            ),
            json!({"server_ips": server_ips}),
        )
        .await
    }

    /// Look up server IP addresses by a nonempty list of nonzero Steam IDs.
    /// JSON decimal strings preserve the full unsigned 64-bit range.
    pub async fn get_server_ips_by_steam_id(
        &self,
        server_steamids: &[SteamId],
    ) -> Result<OpaqueResponse> {
        if server_steamids.is_empty() || server_steamids.iter().any(|id| id.raw() == 0) {
            return Err(Error::InvalidParameter(
                "server_steamids must contain nonzero Steam IDs",
            ));
        }
        self.call(
            Endpoint::get(
                "IGameServersService/GetServerIPsBySteamID/v1/",
                Access::User,
            ),
            json!({"server_steamids": server_steamids}),
        )
        .await
    }

    async fn call(&self, endpoint: Endpoint, payload: Value) -> Result<OpaqueResponse> {
        self.0
            .call(endpoint, vec![param("input_json", payload)], &[])
            .await
    }
}

fn validate_memo(memo: &str) -> Result<()> {
    if memo.contains('\0') {
        Err(Error::InvalidParameter("memo must contain no NUL"))
    } else {
        Ok(())
    }
}
