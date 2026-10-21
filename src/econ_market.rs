// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! The four documented publisher `IEconMarketService` methods.
//!
//! Requests use `input_json` and require a publisher key; market eligibility
//! additionally requires Economy permissions. The
//! [reference](https://partner.steamgames.com/doc/webapi/IEconMarketService) does
//! not specify complete response schemas. Results remain opaque with redacted
//! Debug output, preserving price representations and unknown fields.

use crate::{
    Error, Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::{nonzero, text},
    user_auth::OpaqueResponse,
};
use serde::Serialize;

/// Access to publisher market eligibility, listings, and popular assets.
#[derive(Debug, Clone, Copy)]
pub struct EconMarketService<'a>(pub(crate) &'a SteamClient);

/// One offset-based popular-market-items query.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct PopularMarketOptions<'a> {
    /// Steam API language used for asset descriptions.
    pub language: &'a str,
    /// Optional positive page size; Steam chooses its default when omitted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<u32>,
    /// Initial result offset, or the next offset chosen from the previous page.
    pub start: u32,
    /// Optional application filter. Omission leaves the app unrestricted.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_appid: Option<u32>,
    /// Optional positive Steam currency identifier; no conversion is performed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ecurrency: Option<u32>,
}

impl EconMarketService<'_> {
    /// Check a user's market eligibility (publisher GET v1).
    /// A successful request does not imply eligibility; inspect Steam's result.
    pub async fn get_market_eligibility(&self, steamid: SteamId) -> Result<OpaqueResponse> {
        nonzero(steamid.raw(), "steamid must not be zero")?;
        #[derive(Serialize)]
        struct Request {
            steamid: SteamId,
        }
        self.0
            .call_opaque(
                Endpoint::get(
                    "IEconMarketService/GetMarketEligibility/v1/",
                    Access::Publisher,
                ),
                Params::json(&Request { steamid })?,
            )
            .await
    }

    /// Cancel every listing for one user and application (publisher POST v1).
    /// Sent once. `synchronous` asks Steam to wait for cancellation completion.
    pub async fn cancel_app_listings_for_user(
        &self,
        appid: u32,
        steamid: SteamId,
        synchronous: bool,
    ) -> Result<OpaqueResponse> {
        nonzero(appid.into(), "appid must not be zero")?;
        nonzero(steamid.raw(), "steamid must not be zero")?;
        #[derive(Serialize)]
        struct Request {
            appid: u32,
            steamid: SteamId,
            synchronous: bool,
        }
        self.0
            .call_opaque(
                Endpoint::post(
                    "IEconMarketService/CancelAppListingsForUser/v1/",
                    Access::Publisher,
                ),
                Params::json(&Request {
                    appid,
                    steamid,
                    synchronous,
                })?,
            )
            .await
    }

    /// Read the asset ID belonging to a sold listing (publisher GET v1).
    /// Both application ownership and listing/app agreement are checked by Steam.
    pub async fn get_asset_id(&self, appid: u32, listingid: u64) -> Result<OpaqueResponse> {
        nonzero(appid.into(), "appid must not be zero")?;
        nonzero(listingid, "listingid must not be zero")?;
        #[derive(Serialize)]
        struct Request {
            appid: u32,
            listingid: u64,
        }
        self.0
            .call_opaque(
                Endpoint::get("IEconMarketService/GetAssetID/v1/", Access::Publisher),
                Params::json(&Request { appid, listingid })?,
            )
            .await
    }

    /// Read one popular-items page (publisher GET v1).
    /// Offsets and opaque response price data are returned without inference.
    pub async fn get_popular(&self, options: PopularMarketOptions<'_>) -> Result<OpaqueResponse> {
        text(
            options.language,
            "language must not be empty or contain NUL",
        )?;
        if options.rows == Some(0)
            || options.filter_appid == Some(0)
            || options.ecurrency == Some(0)
        {
            return Err(Error::InvalidParameter(
                "rows, filter_appid and ecurrency must be positive when supplied",
            ));
        }
        self.0
            .call_opaque(
                Endpoint::get("IEconMarketService/GetPopular/v1/", Access::Publisher),
                Params::json(&options)?,
            )
            .await
    }
}
