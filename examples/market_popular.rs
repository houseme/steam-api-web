// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read one page of popular market assets for an application.
//!
//! Run with `cargo run --example market_popular -- <appid>`.

use steam_api_web::econ_market::PopularMarketOptions;
use steam_api_web::{ApiKey, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args
        .next()
        .ok_or("usage: market_popular <appid>")?
        .parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let response = client
        .econ_market()
        .get_popular(PopularMarketOptions {
            language: "english",
            rows: Some(20),
            start: 0,
            filter_appid: Some(appid),
            ecurrency: None,
        })
        .await?;
    println!("Popular assets: {response:?}");
    Ok(())
}
