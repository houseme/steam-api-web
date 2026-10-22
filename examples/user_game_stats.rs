// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read game definitions, user stats, and achievements without writing statistic values.
//!
//! Run with `cargo run --example user_game_stats -- <appid> <steamid>`.

use steam_api_web::{ApiKey, Host, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args
        .next()
        .ok_or("usage: user_game_stats <appid> <steamid>")?
        .parse()?;
    let steamid: u64 = args
        .next()
        .ok_or("usage: user_game_stats <appid> <steamid>")?
        .parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::user(std::env::var("STEAM_USER_KEY")?)?;
    let client = SteamClient::builder()
        .host(Host::Public)
        .api_key(key)
        .build()?;
    let schema = client
        .user_stats()
        .get_schema_for_game(appid, Some("english"))
        .await?;
    let stats = client
        .user_stats()
        .get_user_stats_for_game(steamid, appid)
        .await?;
    let achievements = client
        .user_stats()
        .get_player_achievements(steamid, appid, Some("english"))
        .await?;
    // Numeric StatValue variants preserve exact integer values rather than forcing f64.
    println!(
        "Stats: {}; achievement entries: {}",
        stats.stats.len(),
        achievements.achievements.len()
    );
    let _ = schema;
    Ok(())
}
