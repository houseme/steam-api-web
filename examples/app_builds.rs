// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read one owned application's build history without changing its live build.
//!
//! Run with `cargo run --example app_builds -- <appid>`.

use steam_api_web::{ApiKey, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args.next().ok_or("usage: app_builds <appid>")?.parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let response = client.apps().get_app_builds(appid, Some(5)).await?;
    // Inspect response.as_json() for the app-specific build records.
    println!("Build history: {response:?}");
    Ok(())
}
