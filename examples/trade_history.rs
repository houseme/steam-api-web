// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read the first trade-history page using an exact timestamp and trade-ID cursor.
//!
//! Run with `cargo run --example trade_history -- `.

use steam_api_web::econ::TradeHistoryOptions;
use steam_api_web::{ApiKey, Host, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = ApiKey::user(std::env::var("STEAM_USER_KEY")?)?;
    let client = SteamClient::builder()
        .host(Host::Public)
        .api_key(key)
        .build()?;
    let response = client
        .econ()
        .get_trade_history(TradeHistoryOptions {
            max_trades: 20,
            start_after_time: 0,
            start_after_tradeid: 0,
            navigating_back: false,
            get_descriptions: true,
            language: "english",
            include_failed: false,
            include_total: true,
        })
        .await?;
    // Read continuation and trade outcomes from response.as_json(); do not narrow IDs to f64.
    println!("Trade history: {response:?}");
    Ok(())
}
