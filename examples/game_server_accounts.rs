// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! List game-server accounts without creating accounts or rotating login tokens.
//!
//! Run with `cargo run --example game_server_accounts -- `.

use steam_api_web::{ApiKey, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let response = client.game_servers().get_account_list().await?;
    // Account responses can contain login tokens; Debug deliberately redacts the payload.
    println!("Game-server accounts: {response:?}");
    Ok(())
}
