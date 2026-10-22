// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! List notification sessions without creating sessions or sending notifications.
//!
//! Run with `cargo run --example notification_sessions -- <appid> <steamid>`.

use steam_api_web::game_notifications::EnumerateSessionsRequest;
use steam_api_web::{ApiKey, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args
        .next()
        .ok_or("usage: notification_sessions <appid> <steamid>")?
        .parse()?;
    let steamid: u64 = args
        .next()
        .ok_or("usage: notification_sessions <appid> <steamid>")?
        .parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let response = client
        .game_notifications()
        .enumerate_sessions_for_app(&EnumerateSessionsRequest {
            steamid: steamid.into(),
            appid: Some(appid),
            include_all_user_messages: Some(false),
            include_auth_user_message: Some(false),
            language: Some("english"),
        })
        .await?;
    println!("Notification sessions: {response:?}");
    Ok(())
}
