// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read server statistics using Seattle local times supplied by the caller.
//!
//! Run with `cargo run --example game_server_stats -- <appid> <gameid> <rangestart> <rangeend>`.

use steam_api_web::game_server_stats::GameServerPlayerStatsRequest;
use steam_api_web::{ApiKey, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args
        .next()
        .ok_or("usage: game_server_stats <appid> <gameid> <rangestart> <rangeend>")?
        .parse()?;
    let gameid: u64 = args
        .next()
        .ok_or("usage: game_server_stats <appid> <gameid> <rangestart> <rangeend>")?
        .parse()?;
    let rangestart: String = args
        .next()
        .ok_or("usage: game_server_stats <appid> <gameid> <rangestart> <rangeend>")?;
    let rangeend: String = args
        .next()
        .ok_or("usage: game_server_stats <appid> <gameid> <rangestart> <rangeend>")?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let response = client
        .game_server_stats()
        .get_game_server_player_stats_for_game(&GameServerPlayerStatsRequest {
            gameid,
            appid,
            rangestart: &rangestart,
            rangeend: &rangeend,
            maxresults: Some(100),
        })
        .await?;
    // Quote each "YYYY-MM-DD HH:MM:SS" argument; these values are not UTC timestamps.
    println!("Server statistics: {response:?}");
    Ok(())
}
