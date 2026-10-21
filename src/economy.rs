// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! The eight documented `ISteamEconomy` methods.
//!
//! These endpoints use ordinary form parameters. Class and asset arrays use
//! suffixes such as `classid0`, rather than bracketed array indexes. The
//! [reference](https://partner.steamgames.com/doc/webapi/ISteamEconomy) does not
//! document complete result schemas, so results retain opaque, redacted JSON.
//! Publisher mutations are sent once, including the GET-based `StartTrade`.

use crate::{
    Error, Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::{nonzero, text},
    user_auth::OpaqueResponse,
};
use std::net::IpAddr;

/// Access to Steam economy asset prices, purchases, and trading.
#[derive(Debug, Clone, Copy)]
pub struct Economy<'a>(pub(crate) &'a SteamClient);

/// One requested asset class, optionally identifying a specific instance.
#[derive(Debug, Clone, Copy)]
pub struct AssetClass {
    /// Nonzero class ID, transmitted without floating-point conversion.
    pub classid: u64,
    /// Instance ID; zero is valid for a class without a specialized instance.
    pub instanceid: Option<u64>,
}

/// Optional localization and currency filters for asset price reads.
#[derive(Debug, Clone, Copy, Default)]
pub struct AssetPriceOptions<'a> {
    /// Currency name understood by Steam, such as `USD`.
    pub currency: Option<&'a str>,
    /// Steam API language name.
    pub language: Option<&'a str>,
}

/// One asset and quantity in a purchase initiation request.
#[derive(Debug, Clone, Copy)]
pub struct AssetPurchase<'a> {
    /// Opaque asset identifier; its original spelling is preserved.
    pub assetid: &'a str,
    /// Positive quantity of this asset.
    pub quantity: u32,
}

/// Borrowed parameters for initiating an asset purchase.
#[derive(Debug, Clone, Copy)]
pub struct StartAssetTransactionRequest<'a> {
    /// Application associated with the publisher key.
    pub appid: u32,
    /// Purchasing user.
    pub steamid: SteamId,
    /// At least one asset with a positive quantity.
    pub assets: &'a [AssetPurchase<'a>],
    /// User's local currency. The crate performs no price conversions.
    pub currency: &'a str,
    /// User's Steam API language.
    pub language: &'a str,
    /// User's IPv4 or IPv6 address, without a port or hostname.
    pub ipaddress: &'a str,
    /// Optional referring URL.
    pub referrer: Option<&'a str>,
    /// Request Steam client overlay authorization instead of a web page.
    pub clientauth: Option<bool>,
}

impl Economy<'_> {
    /// Check trade eligibility (publisher GET v1). The original result is opaque;
    /// transport success does not establish that the users may trade.
    pub async fn can_trade(
        &self,
        appid: u32,
        steamid: SteamId,
        targetid: SteamId,
    ) -> Result<OpaqueResponse> {
        validate_user_app(appid, steamid)?;
        nonzero(targetid.raw(), "targetid must not be zero")?;
        self.0
            .call_opaque(
                Endpoint::get("ISteamEconomy/CanTrade/v1/", Access::Publisher),
                Params::new()
                    .with("appid", appid)
                    .with("steamid", steamid)
                    .with("targetid", targetid),
            )
            .await
    }

    /// Finalize the exact opaque transaction ID supplied by Steam (publisher
    /// POST v1). Never automatically replayed; no purchase success is inferred.
    pub async fn finalize_asset_transaction(
        &self,
        appid: u32,
        steamid: SteamId,
        txnid: &str,
        language: &str,
    ) -> Result<OpaqueResponse> {
        validate_user_app(appid, steamid)?;
        text(txnid, "txnid must not be empty or contain NUL")?;
        text(language, "language must not be empty or contain NUL")?;
        self.0
            .call_opaque(
                Endpoint::post(
                    "ISteamEconomy/FinalizeAssetTransaction/v1/",
                    Access::Publisher,
                ),
                Params::new()
                    .with("appid", appid)
                    .with("steamid", steamid)
                    .with("txnid", txnid)
                    .with("language", language),
            )
            .await
    }

    /// Read one or more asset classes using a user or publisher key (GET v1).
    /// The wire format is `class_count`, `classid0`, and optional `instanceid0`.
    pub async fn get_asset_class_info(
        &self,
        appid: u32,
        classes: &[AssetClass],
        language: Option<&str>,
    ) -> Result<OpaqueResponse> {
        nonzero(appid.into(), "appid must not be zero")?;
        let count = u32::try_from(classes.len())
            .map_err(|_| Error::InvalidParameter("too many asset classes"))?;
        nonzero(count.into(), "at least one asset class is required")?;
        let mut params = Params::new()
            .with("appid", appid)
            .with("class_count", count);
        if let Some(language) = language {
            text(language, "language must not be empty or contain NUL")?;
            params.push("language", language);
        }
        for (index, class) in classes.iter().enumerate() {
            nonzero(class.classid, "classid must not be zero")?;
            params.push(&format!("classid{index}"), class.classid);
            if let Some(instanceid) = class.instanceid {
                params.push(&format!("instanceid{index}"), instanceid);
            }
        }
        self.0
            .call_opaque(
                Endpoint::get("ISteamEconomy/GetAssetClassInfo/v1/", Access::User),
                params,
            )
            .await
    }

    /// Read purchasable asset prices and categories (user-key GET v1).
    /// Original numeric and textual price representations remain in the JSON.
    pub async fn get_asset_prices(
        &self,
        appid: u32,
        options: AssetPriceOptions<'_>,
    ) -> Result<OpaqueResponse> {
        nonzero(appid.into(), "appid must not be zero")?;
        let mut params = Params::new().with("appid", appid);
        for (name, value) in [
            ("currency", options.currency),
            ("language", options.language),
        ] {
            if let Some(value) = value {
                text(value, "price filter must not be empty or contain NUL")?;
                params.push(name, value);
            }
        }
        self.0
            .call_opaque(
                Endpoint::get("ISteamEconomy/GetAssetPrices/v1/", Access::User),
                params,
            )
            .await
    }

    /// Read exported assets for a user's app context (publisher GET v1).
    pub async fn get_exported_assets_for_user(
        &self,
        appid: u32,
        steamid: SteamId,
        contextid: u64,
    ) -> Result<OpaqueResponse> {
        validate_user_app(appid, steamid)?;
        nonzero(contextid, "contextid must not be zero")?;
        self.0
            .call_opaque(
                Endpoint::get(
                    "ISteamEconomy/GetExportedAssetsForUser/v1/",
                    Access::Publisher,
                ),
                Params::new()
                    .with("appid", appid)
                    .with("steamid", steamid)
                    .with("contextid", contextid),
            )
            .await
    }

    /// Read a game's market prices with a publisher key (GET v1).
    /// No floating-point price conversion is performed by this method.
    pub async fn get_market_prices(&self, appid: u32) -> Result<OpaqueResponse> {
        nonzero(appid.into(), "appid must not be zero")?;
        self.0
            .call_opaque(
                Endpoint::get("ISteamEconomy/GetMarketPrices/v1/", Access::Publisher),
                Params::new().with("appid", appid),
            )
            .await
    }

    /// Initiate an asset purchase (publisher POST v1), sent exactly once.
    /// Authorization and finalization remain separate application decisions.
    /// Valve documents no caller-provided idempotency key for this endpoint.
    pub async fn start_asset_transaction(
        &self,
        request: StartAssetTransactionRequest<'_>,
    ) -> Result<OpaqueResponse> {
        validate_user_app(request.appid, request.steamid)?;
        if request.assets.is_empty() || u32::try_from(request.assets.len()).is_err() {
            return Err(Error::InvalidParameter(
                "assets must contain a uint32-sized nonempty list",
            ));
        }
        text(
            request.currency,
            "currency must not be empty or contain NUL",
        )?;
        text(
            request.language,
            "language must not be empty or contain NUL",
        )?;
        if request.ipaddress.parse::<IpAddr>().is_err() {
            return Err(Error::InvalidParameter(
                "ipaddress must be an IP literal without a port",
            ));
        }
        let mut params = Params::new()
            .with("appid", request.appid)
            .with("steamid", request.steamid)
            .with("currency", request.currency)
            .with("language", request.language)
            .with("ipaddress", request.ipaddress);
        for (index, asset) in request.assets.iter().enumerate() {
            text(asset.assetid, "assetid must not be empty or contain NUL")?;
            nonzero(asset.quantity.into(), "asset quantity must not be zero")?;
            params.push(&format!("assetid{index}"), asset.assetid);
            params.push(&format!("assetquantity{index}"), asset.quantity);
        }
        if let Some(referrer) = request.referrer {
            text(referrer, "referrer must not be empty or contain NUL")?;
            params.push("referrer", referrer);
        }
        if let Some(clientauth) = request.clientauth {
            params.push("clientauth", clientauth);
        }
        self.0
            .call_opaque(
                Endpoint::post("ISteamEconomy/StartAssetTransaction/v1/", Access::Publisher),
                params,
            )
            .await
    }

    /// Initiate a trade through Valve's GET mutation (publisher v1).
    /// Never automatically replayed, including after HTTP 429 or 503.
    pub async fn start_trade(
        &self,
        appid: u32,
        partya: SteamId,
        partyb: SteamId,
    ) -> Result<OpaqueResponse> {
        validate_user_app(appid, partya)?;
        nonzero(partyb.raw(), "partyb must not be zero")?;
        if partya == partyb {
            return Err(Error::InvalidParameter("trade parties must be distinct"));
        }
        self.0
            .call_opaque(
                Endpoint::get_once("ISteamEconomy/StartTrade/v1/", Access::Publisher),
                Params::new()
                    .with("appid", appid)
                    .with("partya", partya)
                    .with("partyb", partyb),
            )
            .await
    }
}

fn validate_user_app(appid: u32, steamid: SteamId) -> Result<()> {
    nonzero(appid.into(), "appid must not be zero")?;
    nonzero(steamid.raw(), "steamid must not be zero")
}
