// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Query an anonymous public endpoint: cargo run --example current_players -- 480
use steam_api_web::SteamClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let appid = std::env::args()
        .nth(1)
        .map_or(Ok(480), |value| value.parse::<u32>())?;
    let client = SteamClient::builder().build()?;
    let result = client
        .user_stats()
        .get_number_of_current_players(appid)
        .await?;
    println!("App {appid}: {} connected players", result.player_count);
    Ok(())
}
