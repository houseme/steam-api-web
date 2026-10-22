// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Submit one explicit community abuse report; this performs a write.
//!
//! Run with `cargo run --example community_report -- <appid> <actor> <target> <abuse_type> <content_type> <description_file>`.

use steam_api_web::community::ReportAbuseRequest;
use steam_api_web::{ApiKey, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args.next().ok_or("usage: community_report <appid> <actor> <target> <abuse_type> <content_type> <description_file>")?.parse()?;
    let actor: u64 = args.next().ok_or("usage: community_report <appid> <actor> <target> <abuse_type> <content_type> <description_file>")?.parse()?;
    let target: u64 = args.next().ok_or("usage: community_report <appid> <actor> <target> <abuse_type> <content_type> <description_file>")?.parse()?;
    let abuse_type: u32 = args.next().ok_or("usage: community_report <appid> <actor> <target> <abuse_type> <content_type> <description_file>")?.parse()?;
    let content_type: u32 = args.next().ok_or("usage: community_report <appid> <actor> <target> <abuse_type> <content_type> <description_file>")?.parse()?;
    let description_file: String = args.next().ok_or("usage: community_report <appid> <actor> <target> <abuse_type> <content_type> <description_file>")?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let description = std::fs::read_to_string(description_file)?;
    let response = client
        .community()
        .report_abuse(&ReportAbuseRequest {
            steamid_actor: actor.into(),
            steamid_target: target.into(),
            appid,
            abuse_type,
            content_type,
            description: &description,
            gid: None,
        })
        .await?;
    // Supply the numeric abuse/content categories from the applicable Steam contract.
    println!("Report response: {response:?}");
    Ok(())
}
