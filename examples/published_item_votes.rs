// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read vote summaries for one published file using the documented POST.
//!
//! Run with `cargo run --example published_item_votes -- <appid> <steamid> <publishedfileid>`.

use steam_api_web::{ApiKey, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args
        .next()
        .ok_or("usage: published_item_votes <appid> <steamid> <publishedfileid>")?
        .parse()?;
    let steamid: u64 = args
        .next()
        .ok_or("usage: published_item_votes <appid> <steamid> <publishedfileid>")?
        .parse()?;
    let publishedfileid: u64 = args
        .next()
        .ok_or("usage: published_item_votes <appid> <steamid> <publishedfileid>")?
        .parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let response = client
        .published_item_voting()
        .item_vote_summary(steamid.into(), appid, &[publishedfileid])
        .await?;
    println!("Vote summary: {response:?}");
    Ok(())
}
