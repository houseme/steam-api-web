// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read one lobby without creating it or changing its membership.
//!
//! Run with `cargo run --example lobby_data -- <appid> <lobby_id>`.

use steam_api_web::{ApiKey, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args
        .next()
        .ok_or("usage: lobby_data <appid> <lobby_id>")?
        .parse()?;
    let lobby_id: u64 = args
        .next()
        .ok_or("usage: lobby_data <appid> <lobby_id>")?
        .parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let response = client
        .lobby_matchmaking()
        .get_lobby_data(appid, lobby_id.into())
        .await?;
    // Lobby metadata is distinct from notification sessions and native SDK callbacks.
    println!("Lobby: {response:?}");
    Ok(())
}
