// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! ISteamWebAPIUtil: server metadata and discovery of supported methods.
//!
//! The official reference does not specify either response schema. These methods
//! preserve the complete JSON document rather than requiring an inferred shape.

use crate::{
    Result, SteamClient,
    request::{Access, Endpoint, Params},
    user_auth::OpaqueResponse,
};

/// Access to ISteamWebAPIUtil methods.
#[derive(Debug, Clone, Copy)]
pub struct WebApiUtil<'a>(pub(crate) &'a SteamClient);

impl WebApiUtil<'_> {
    /// Read server metadata without requiring an API key.
    ///
    /// # HTTP contract
    ///
    /// [GetServerInfo](https://partner.steamgames.com/doc/webapi/ISteamWebAPIUtil#GetServerInfo): `GET ISteamWebAPIUtil/GetServerInfo/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/api_discovery.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(client: &SteamClient) -> steam_api_web::Result<()> {
    ///     let response = client.web_api_util().get_server_info().await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_server_info(&self) -> Result<OpaqueResponse> {
        self.0
            .call_opaque(
                Endpoint::get("ISteamWebAPIUtil/GetServerInfo/v1/", Access::Anonymous),
                Params::new(),
            )
            .await
    }

    /// Discover supported API methods. A configured API key is sent by the client
    /// and may expose restricted methods; anonymous calls return public methods.
    ///
    /// # HTTP contract
    ///
    /// [GetSupportedAPIList](https://partner.steamgames.com/doc/webapi/ISteamWebAPIUtil#GetSupportedAPIList): `GET ISteamWebAPIUtil/GetSupportedAPIList/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/api_discovery.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(client: &SteamClient) -> steam_api_web::Result<()> {
    ///     let response = client.web_api_util().get_supported_api_list().await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_supported_api_list(&self) -> Result<OpaqueResponse> {
        self.0
            .call_opaque(
                Endpoint::get(
                    "ISteamWebAPIUtil/GetSupportedAPIList/v1/",
                    Access::Anonymous,
                ),
                Params::new(),
            )
            .await
    }
}
