// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read a bounded report interval without issuing bans or starting VAC sessions.
//!
//! Run with `cargo run --example cheating_reports -- <appid> <timebegin> <timeend>`.

use steam_api_web::cheat_reporting::CheatingReportsRequest;
use steam_api_web::{ApiKey, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args
        .next()
        .ok_or("usage: cheating_reports <appid> <timebegin> <timeend>")?
        .parse()?;
    let timebegin: u32 = args
        .next()
        .ok_or("usage: cheating_reports <appid> <timebegin> <timeend>")?
        .parse()?;
    let timeend: u32 = args
        .next()
        .ok_or("usage: cheating_reports <appid> <timebegin> <timeend>")?
        .parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let response = client
        .cheat_reporting()
        .get_cheating_reports(&CheatingReportsRequest {
            appid,
            timebegin,
            timeend,
            reportidmin: 0,
            includereports: Some(true),
            includebans: Some(true),
            steamid: None,
        })
        .await?;
    // Persist a report boundary only after processing the corresponding records.
    println!("Reports: {response:?}");
    Ok(())
}
