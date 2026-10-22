// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read one filtered banned-player page with an optional continuation token.
//!
//! Run `cargo run --example banned_players -- <appid> <starttime> <endtime> [continue_token]`.
//! Set STEAM_PUBLISHER_KEY; timestamps are Unix seconds and must stay unchanged between pages.

use steam_api_web::{ApiKey, SteamClient, apps::BannedPlayersOptions};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid = args.next().ok_or("missing appid")?.parse()?;
    let starttime = args.next().ok_or("missing starttime")?.parse()?;
    let endtime = args.next().ok_or("missing endtime")?.parse()?;
    let continue_token = args.next();
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let response = client
        .apps()
        .get_players_banned_with_options(
            appid,
            BannedPlayersOptions {
                starttime: Some(starttime),
                endtime: Some(endtime),
                continue_token: continue_token.as_deref(),
            },
        )
        .await?;
    // Inspect response.as_json() for ban records and the documented continuation.
    // No loop is inferred from an undocumented response envelope or item count.
    println!("Banned-player page: {response:?}");
    Ok(())
}
