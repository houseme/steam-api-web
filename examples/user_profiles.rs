// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read one public profile and ban summary using a User key.
//!
//! Run with `cargo run --example user_profiles -- <steamid>`.

use steam_api_web::{ApiKey, Host, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let steamid: u64 = args
        .next()
        .ok_or("usage: user_profiles <steamid>")?
        .parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::user(std::env::var("STEAM_USER_KEY")?)?;
    let client = SteamClient::builder()
        .host(Host::Public)
        .api_key(key)
        .build()?;
    let ids = [steamid.into()];
    let profiles = client.user().get_player_summaries(&ids).await?;
    let bans = client.user().get_player_bans(&ids).await?;
    // Steam may omit unknown users; successful reads do not establish app ownership.
    println!(
        "Profiles: {}; ban summaries: {}",
        profiles.len(),
        bans.len()
    );
    Ok(())
}
