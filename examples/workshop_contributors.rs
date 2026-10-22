// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read finalized Workshop contributors without changing payment rules.
//!
//! Run with `cargo run --example workshop_contributors -- <appid> <gameitemid>`.

use steam_api_web::{ApiKey, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args
        .next()
        .ok_or("usage: workshop_contributors <appid> <gameitemid>")?
        .parse()?;
    let gameitemid: u32 = args
        .next()
        .ok_or("usage: workshop_contributors <appid> <gameitemid>")?
        .parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let response = client
        .workshop()
        .get_finalized_contributors(appid, gameitemid)
        .await?;
    // Check the applicable partner contract before interpreting contributor-specific fields.
    println!("Finalized contributors: {response:?}");
    Ok(())
}
