// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Trade history, offers, and publisher cache administration (`IEconService`).
//!
//! All seven methods use `input_json`. The
//! [reference](https://partner.steamgames.com/doc/webapi/IEconService) omits complete
//! result schemas; responses preserve opaque JSON with redacted Debug output.
//! Trade reads require a user or publisher key. Cache mutations require a
//! publisher key with Economy permissions and are never automatically replayed.

use crate::{
    Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::{nonzero, text},
    user_auth::OpaqueResponse,
};
use serde::Serialize;

/// Access to the seven documented Steam trading service methods.
#[derive(Debug, Clone, Copy)]
pub struct EconService<'a>(pub(crate) &'a SteamClient);

/// A single trade-history page query, retaining Steam's time/ID continuation.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct TradeHistoryOptions<'a> {
    /// Positive maximum number of trades. Valve documents no maximum here.
    pub max_trades: u32,
    /// Timestamp from the previous page; zero starts the initial page.
    pub start_after_time: u32,
    /// Exact trade ID from the previous page; zero starts the initial page.
    /// Serialized as a JSON uint64 without floating-point conversion.
    pub start_after_tradeid: u64,
    /// Navigate backwards from the first trade of the current page.
    pub navigating_back: bool,
    /// Include display data for returned assets.
    pub get_descriptions: bool,
    /// Steam API language name.
    pub language: &'a str,
    /// Include failed trades.
    pub include_failed: bool,
    /// Include the account's total trade count.
    pub include_total: bool,
}

/// Sent/received offer selection and historical filters.
#[derive(Debug, Clone, Copy, Serialize)]
pub struct TradeOffersOptions<'a> {
    /// Request sent offers.
    pub get_sent_offers: bool,
    /// Request received offers. Both direction flags are forwarded unchanged.
    pub get_received_offers: bool,
    /// Include item display information.
    pub get_descriptions: bool,
    /// Steam API language name.
    pub language: &'a str,
    /// Return active offers and offers updated since the cutoff.
    pub active_only: bool,
    /// Return inactive offers. Steam applies this alongside the active/cutoff filter.
    pub historical_only: bool,
    /// Unix timestamp used by `active_only`; zero includes all history.
    pub time_historical_cutoff: u32,
}

impl EconService<'_> {
    /// Read one trade-history page using the exact timestamp/trade-ID cursor.
    /// Callers inspect the opaque response for continuation and trade outcomes.
    pub async fn get_trade_history(
        &self,
        options: TradeHistoryOptions<'_>,
    ) -> Result<OpaqueResponse> {
        nonzero(options.max_trades.into(), "max_trades must not be zero")?;
        text(
            options.language,
            "language must not be empty or contain NUL",
        )?;
        self.0
            .call_opaque(
                Endpoint::get("IEconService/GetTradeHistory/v1/", Access::User),
                Params::json(&options)?,
            )
            .await
    }

    /// Flush a user's inventory cache (publisher POST v1), without replay.
    pub async fn flush_inventory_cache(
        &self,
        steamid: SteamId,
        appid: u32,
        contextid: u64,
    ) -> Result<OpaqueResponse> {
        nonzero(steamid.raw(), "steamid must not be zero")?;
        nonzero(appid.into(), "appid must not be zero")?;
        nonzero(contextid, "contextid must not be zero")?;
        #[derive(Serialize)]
        struct Request {
            steamid: SteamId,
            appid: u32,
            contextid: u64,
        }
        self.0
            .call_opaque(
                Endpoint::post("IEconService/FlushInventoryCache/v1/", Access::Publisher),
                Params::json(&Request {
                    steamid,
                    appid,
                    contextid,
                })?,
            )
            .await
    }

    /// Invalidate the game's asset appearance cache (publisher POST v1).
    pub async fn flush_asset_appearance_cache(&self, appid: u32) -> Result<OpaqueResponse> {
        self.flush_app("IEconService/FlushAssetAppearanceCache/v1/", appid)
            .await
    }

    /// Invalidate cached inventory contents (publisher POST v1), without replay.
    pub async fn flush_context_cache(&self, appid: u32) -> Result<OpaqueResponse> {
        self.flush_app("IEconService/FlushContextCache/v1/", appid)
            .await
    }

    /// Read sent and/or received offers (user-key GET v1).
    /// No undocumented pagination cursor is synthesized by this binding.
    pub async fn get_trade_offers(
        &self,
        options: TradeOffersOptions<'_>,
    ) -> Result<OpaqueResponse> {
        text(
            options.language,
            "language must not be empty or contain NUL",
        )?;
        self.0
            .call_opaque(
                Endpoint::get("IEconService/GetTradeOffers/v1/", Access::User),
                Params::json(&options)?,
            )
            .await
    }

    /// Read one exact 64-bit trade offer ID (user-key GET v1).
    pub async fn get_trade_offer(
        &self,
        tradeofferid: u64,
        language: &str,
    ) -> Result<OpaqueResponse> {
        nonzero(tradeofferid, "tradeofferid must not be zero")?;
        text(language, "language must not be empty or contain NUL")?;
        #[derive(Serialize)]
        struct Request<'a> {
            tradeofferid: u64,
            language: &'a str,
        }
        self.0
            .call_opaque(
                Endpoint::get("IEconService/GetTradeOffer/v1/", Access::User),
                Params::json(&Request {
                    tradeofferid,
                    language,
                })?,
            )
            .await
    }

    /// Read pending/new offer counts (user-key GET v1). Omit `time_last_visit`
    /// to use the previous trade-offer page visit, as described by Valve.
    pub async fn get_trade_offers_summary(
        &self,
        time_last_visit: Option<u32>,
    ) -> Result<OpaqueResponse> {
        #[derive(Serialize)]
        struct Request {
            #[serde(skip_serializing_if = "Option::is_none")]
            time_last_visit: Option<u32>,
        }
        self.0
            .call_opaque(
                Endpoint::get("IEconService/GetTradeOffersSummary/v1/", Access::User),
                Params::json(&Request { time_last_visit })?,
            )
            .await
    }

    async fn flush_app(&self, path: &'static str, appid: u32) -> Result<OpaqueResponse> {
        nonzero(appid.into(), "appid must not be zero")?;
        #[derive(Serialize)]
        struct Request {
            appid: u32,
        }
        self.0
            .call_opaque(
                Endpoint::post(path, Access::Publisher),
                Params::json(&Request { appid })?,
            )
            .await
    }
}
