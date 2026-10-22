// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Explicitly read a production transaction without creating or finalizing payments.
//!
//! Run with `cargo run --example query_production_transaction -- <appid> <transid>`.

use steam_api_web::micro_txn::QueryTxnRequest;
use steam_api_web::{ApiKey, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args
        .next()
        .ok_or("usage: query_production_transaction <appid> <transid>")?
        .parse()?;
    let transid: u64 = args
        .next()
        .ok_or("usage: query_production_transaction <appid> <transid>")?
        .parse()?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::publisher(std::env::var("STEAM_PUBLISHER_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let response = client
        .micro_txn_production()
        .query_txn(QueryTxnRequest {
            appid,
            orderid: None,
            transid: Some(transid),
        })
        .await?;
    // This selects production explicitly. For test transactions use query_sandbox_transaction.
    println!("Production query response: {response:?}");
    Ok(())
}
