// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Submit one metadata frame to an existing broadcast; this performs a write.
//!
//! Run with `cargo run --example broadcast_frame -- <appid> <steamid> <broadcast_id> <frame_file>`.

use steam_api_web::broadcast::PostGameDataFrameRequest;
use steam_api_web::{ApiKey, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args
        .next()
        .ok_or("usage: broadcast_frame <appid> <steamid> <broadcast_id> <frame_file>")?
        .parse()?;
    let steamid: u64 = args
        .next()
        .ok_or("usage: broadcast_frame <appid> <steamid> <broadcast_id> <frame_file>")?
        .parse()?;
    let broadcast_id: u64 = args
        .next()
        .ok_or("usage: broadcast_frame <appid> <steamid> <broadcast_id> <frame_file>")?
        .parse()?;
    let frame_file: String = args
        .next()
        .ok_or("usage: broadcast_frame <appid> <steamid> <broadcast_id> <frame_file>")?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let frame_data = std::fs::read_to_string(frame_file)?;
    let response = client
        .broadcast()
        .post_game_data_frame(&PostGameDataFrameRequest {
            appid,
            steamid: steamid.into(),
            broadcast_id,
            frame_data: &frame_data,
        })
        .await?;
    // A transport failure is ambiguous: reconcile the existing broadcast before resending.
    println!("Frame response: {response:?}");
    Ok(())
}
