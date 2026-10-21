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
