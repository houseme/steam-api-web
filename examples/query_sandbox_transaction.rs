// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read one Sandbox transaction without creating, finalizing, or refunding it.

use steam_api_web::{ApiKey, SteamClient, micro_txn::QueryTxnRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args
        .next()
        .ok_or("usage: query_sandbox_transaction <appid> <transid>")?
        .parse()?;
    let transid: u64 = args.next().ok_or("missing Sandbox transid")?.parse()?;
    let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let response = client
        .micro_txn_sandbox()
        .query_txn(QueryTxnRequest {
            appid,
            orderid: None,
            transid: Some(transid),
        })
        .await?;
    // Observe status only. This example never grants entitlements or switches environments.
    let status = response.as_json()["response"]["params"]["status"]
        .as_str()
        .unwrap_or("not returned");
    println!("Sandbox transaction status: {status}");
    Ok(())
}
