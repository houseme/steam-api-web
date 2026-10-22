// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read a visible game library without interpreting private fields as an empty library.
//!
//! Run with `cargo run --example player_library -- <steamid>`.

use steam_api_web::player::OwnedGamesOptions;
use steam_api_web::{ApiKey, Host, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let steamid: u64 = args
        .next()
        .ok_or("usage: player_library <steamid>")?
        .parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::user(std::env::var("STEAM_USER_KEY")?)?;
    let client = SteamClient::builder()
        .host(Host::Public)
        .api_key(key)
        .build()?;
    let response = client
        .player()
        .get_owned_games(
            steamid,
            OwnedGamesOptions {
                include_appinfo: true,
                include_played_free_games: true,
                appids_filter: &[],
            },
        )
        .await?;
    // Missing response fields may reflect account privacy rather than zero ownership.
    println!("Visible library: {response:?}");
    Ok(())
}
