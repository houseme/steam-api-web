// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Server-side verification. Read credentials from the environment, never argv.
use steam_api_web::{ApiKey, SteamClient, user_auth::WebApiTicket};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
    let ticket = WebApiTicket::from_hex(std::env::var("STEAM_TICKET_HEX")?)?;
    let appid = std::env::var("STEAM_APP_ID")?.parse::<u32>()?;
    let identity = std::env::var("STEAM_TICKET_IDENTITY")?;
    let client = SteamClient::builder().api_key(key).build()?;
    let result = client
        .user_auth()
        .authenticate_user_ticket(appid, &ticket, &identity)
        .await?;
    println!("Verified SteamID: {}", result.steamid);
    // Apply your application's VAC/publisher-ban and license policy here.
    Ok(())
}
