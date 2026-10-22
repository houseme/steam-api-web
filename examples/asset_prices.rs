// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read asset prices using an explicit currency and Steam language.
//!
//! Run with `cargo run --example asset_prices -- <appid>`.

use steam_api_web::economy::AssetPriceOptions;
use steam_api_web::{ApiKey, Host, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args.next().ok_or("usage: asset_prices <appid>")?.parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::user(std::env::var("STEAM_USER_KEY")?)?;
    let client = SteamClient::builder()
        .host(Host::Public)
        .api_key(key)
        .build()?;
    let response = client
        .economy()
        .get_asset_prices(
            appid,
            AssetPriceOptions {
                currency: Some("USD"),
                language: Some("english"),
            },
        )
        .await?;
    // Interpret monetary values in their documented units rather than converting through f64.
    println!("Asset prices: {response:?}");
    Ok(())
}
