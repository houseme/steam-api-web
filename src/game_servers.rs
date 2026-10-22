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
    request::{Access, Endpoint, Params},
    types::{nonzero, text},
    user_auth::OpaqueResponse,
};
use serde::Serialize;
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
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::InvalidParameter`] if the login token is empty or contains a NUL byte.
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
    ///
    /// # HTTP contract
    ///
    /// [GetAccountList](https://partner.steamgames.com/doc/webapi/IGameServersService#GetAccountList): `GET IGameServersService/GetAccountList/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/game_server_accounts.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(client: &SteamClient) -> steam_api_web::Result<()> {
    ///     let response = client.game_servers().get_account_list().await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_account_list(&self) -> Result<OpaqueResponse> {
        self.call_service(
            Endpoint::get("IGameServersService/GetAccountList/v1/", Access::User),
            &EmptyRequest {},
        )
        .await
    }

    /// Create a persistent account. This mutation is never automatically retried.
    /// An empty memo is permitted.
    ///
    /// # HTTP contract
    ///
    /// [CreateAccount](https://partner.steamgames.com/doc/webapi/IGameServersService#CreateAccount): `POST IGameServersService/CreateAccount/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/game_server_accounts.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(client: &SteamClient, appid: u32, memo: &str) -> steam_api_web::Result<()> {
    ///     let response = client.game_servers().create_account(appid, memo).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn create_account(&self, appid: u32, memo: &str) -> Result<OpaqueResponse> {
        nonzero(appid.into(), "appid must not be zero")?;
        validate_memo(memo)?;
        self.call_service(
            Endpoint::post("IGameServersService/CreateAccount/v1/", Access::User),
            &CreateRequest { appid, memo },
        )
        .await
    }

    /// Change or clear an account memo without automatic replay.
    ///
    /// # HTTP contract
    ///
    /// [SetMemo](https://partner.steamgames.com/doc/webapi/IGameServersService#SetMemo): `POST IGameServersService/SetMemo/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/game_server_accounts.rs).
    ///
    /// ```no_run
    /// use steam_api_web::{SteamClient, SteamId};
    ///
    /// async fn example(client: &SteamClient, steamid: SteamId, memo: &str) -> steam_api_web::Result<()> {
    ///     let response = client.game_servers().set_memo(steamid, memo).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn set_memo(&self, steamid: SteamId, memo: &str) -> Result<OpaqueResponse> {
        nonzero(steamid.raw(), "steamid must not be zero")?;
        validate_memo(memo)?;
        self.call_service(
            Endpoint::post("IGameServersService/SetMemo/v1/", Access::User),
            &MemoRequest { steamid, memo },
        )
        .await
    }

    /// Replace an account's login token. Never automatically replayed because a
    /// second successful call could invalidate the token from the first call.
    ///
    /// # HTTP contract
    ///
    /// [ResetLoginToken](https://partner.steamgames.com/doc/webapi/IGameServersService#ResetLoginToken): `POST IGameServersService/ResetLoginToken/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/game_server_accounts.rs).
    ///
    /// ```no_run
    /// use steam_api_web::{SteamClient, SteamId};
    ///
    /// async fn example(client: &SteamClient, steamid: SteamId) -> steam_api_web::Result<()> {
    ///     let response = client.game_servers().reset_login_token(steamid).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn reset_login_token(&self, steamid: SteamId) -> Result<OpaqueResponse> {
        nonzero(steamid.raw(), "steamid must not be zero")?;
        self.call_service(
            Endpoint::post("IGameServersService/ResetLoginToken/v1/", Access::User),
            &ServerRequest { steamid },
        )
        .await
    }

    /// Delete a persistent account without automatic replay.
    ///
    /// # HTTP contract
    ///
    /// [DeleteAccount](https://partner.steamgames.com/doc/webapi/IGameServersService#DeleteAccount): `POST IGameServersService/DeleteAccount/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/game_server_accounts.rs).
    ///
    /// ```no_run
    /// use steam_api_web::{SteamClient, SteamId};
    ///
    /// async fn example(client: &SteamClient, steamid: SteamId) -> steam_api_web::Result<()> {
    ///     let response = client.game_servers().delete_account(steamid).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn delete_account(&self, steamid: SteamId) -> Result<OpaqueResponse> {
        nonzero(steamid.raw(), "steamid must not be zero")?;
        self.call_service(
            Endpoint::post("IGameServersService/DeleteAccount/v1/", Access::User),
            &ServerRequest { steamid },
        )
        .await
    }

    /// Read an account's public information using an authenticated request.
    ///
    /// # HTTP contract
    ///
    /// [GetAccountPublicInfo](https://partner.steamgames.com/doc/webapi/IGameServersService#GetAccountPublicInfo): `GET IGameServersService/GetAccountPublicInfo/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/game_server_accounts.rs).
    ///
    /// ```no_run
    /// use steam_api_web::{SteamClient, SteamId};
    ///
    /// async fn example(client: &SteamClient, steamid: SteamId) -> steam_api_web::Result<()> {
    ///     let response = client.game_servers().get_account_public_info(steamid).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_account_public_info(&self, steamid: SteamId) -> Result<OpaqueResponse> {
        nonzero(steamid.raw(), "steamid must not be zero")?;
        self.call_service(
            Endpoint::get("IGameServersService/GetAccountPublicInfo/v1/", Access::User),
            &ServerRequest { steamid },
        )
        .await
    }

    /// Query the status of a login token owned by the API key's account.
    ///
    /// # HTTP contract
    ///
    /// [QueryLoginToken](https://partner.steamgames.com/doc/webapi/IGameServersService#QueryLoginToken): `GET IGameServersService/QueryLoginToken/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/game_server_accounts.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    /// use steam_api_web::game_servers::GameServerLoginToken;
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     login_token: &GameServerLoginToken,
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client.game_servers().query_login_token(login_token).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn query_login_token(
        &self,
        login_token: &GameServerLoginToken,
    ) -> Result<OpaqueResponse> {
        self.call_service(
            Endpoint::get("IGameServersService/QueryLoginToken/v1/", Access::User),
            &TokenRequest {
                login_token: login_token.expose_secret(),
            },
        )
        .await
    }

    /// Look up server IDs by IP addresses, optionally including a port.
    /// Both IPv4 and IPv6 literals are accepted; hostnames are rejected.
    ///
    /// # HTTP contract
    ///
    /// [GetServerSteamIDsByIP](https://partner.steamgames.com/doc/webapi/IGameServersService#GetServerSteamIDsByIP): `GET IGameServersService/GetServerSteamIDsByIP/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/game_server_accounts.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(client: &SteamClient, server_ips: &[&str]) -> steam_api_web::Result<()> {
    ///     let response = client.game_servers().get_server_steam_ids_by_ip(server_ips).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
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
        self.call_service(
            Endpoint::get(
                "IGameServersService/GetServerSteamIDsByIP/v1/",
                Access::User,
            ),
            &IpsRequest { server_ips },
        )
        .await
    }

    /// Look up server IP addresses by a nonempty list of nonzero Steam IDs.
    /// JSON decimal strings preserve the full unsigned 64-bit range.
    ///
    /// # HTTP contract
    ///
    /// [GetServerIPsBySteamID](https://partner.steamgames.com/doc/webapi/IGameServersService#GetServerIPsBySteamID): `GET IGameServersService/GetServerIPsBySteamID/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/game_server_accounts.rs).
    ///
    /// ```no_run
    /// use steam_api_web::{SteamClient, SteamId};
    ///
    /// async fn example(client: &SteamClient, server_steamids: &[SteamId]) -> steam_api_web::Result<()> {
    ///     let response = client.game_servers().get_server_ips_by_steam_id(server_steamids).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_server_ips_by_steam_id(
        &self,
        server_steamids: &[SteamId],
    ) -> Result<OpaqueResponse> {
        if server_steamids.is_empty() || server_steamids.iter().any(|id| id.raw() == 0) {
            return Err(Error::InvalidParameter(
                "server_steamids must contain nonzero Steam IDs",
            ));
        }
        self.call_service(
            Endpoint::get(
                "IGameServersService/GetServerIPsBySteamID/v1/",
                Access::User,
            ),
            &IdsRequest { server_steamids },
        )
        .await
    }

    async fn call_service(
        &self,
        endpoint: Endpoint,
        payload: &impl Serialize,
    ) -> Result<OpaqueResponse> {
        self.0.call_opaque(endpoint, Params::json(payload)?).await
    }
}

fn validate_memo(memo: &str) -> Result<()> {
    if memo.contains('\0') {
        Err(Error::InvalidParameter("memo must contain no NUL"))
    } else {
        Ok(())
    }
}

#[derive(Serialize)]
struct EmptyRequest {}

#[derive(Serialize)]
struct CreateRequest<'a> {
    appid: u32,
    memo: &'a str,
}

#[derive(Serialize)]
struct MemoRequest<'a> {
    steamid: SteamId,
    memo: &'a str,
}

#[derive(Serialize)]
struct ServerRequest {
    steamid: SteamId,
}

#[derive(Serialize)]
struct TokenRequest<'a> {
    login_token: &'a str,
}

#[derive(Serialize)]
struct IpsRequest<'a, 'b> {
    server_ips: &'a [&'b str],
}

#[derive(Serialize)]
struct IdsRequest<'a> {
    server_steamids: &'a [SteamId],
}
