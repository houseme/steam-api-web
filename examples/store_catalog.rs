// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read a small incremental Steam Store catalogue page.
//!
//! Run with `cargo run --example store_catalog -- `.

use steam_api_web::store::AppListOptions;
use steam_api_web::{ApiKey, Host, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = ApiKey::user(std::env::var("STEAM_USER_KEY")?)?;
    let client = SteamClient::builder()
        .host(Host::Public)
        .api_key(key)
        .build()?;
    let response = client
        .store()
        .get_app_list(AppListOptions {
            max_results: Some(100),
            include_games: Some(true),
            ..Default::default()
        })
        .await?;
    // Preserve the last returned app ID for last_appid; inspect the opaque envelope explicitly.
    println!("Store catalogue: {response:?}");
    Ok(())
}
