// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read one financial sales page and wishlist report without saving reconciliation cursors.
//!
//! Run with `cargo run --example financial_reports -- <appid> <date>`.

use steam_api_web::partner_financials::{AppWishlistRequest, DetailedSalesRequest};
use steam_api_web::{ApiKey, SteamClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let appid: u32 = args
        .next()
        .ok_or("usage: financial_reports <appid> <date>")?
        .parse()?;
    let date: String = args
        .next()
        .ok_or("usage: financial_reports <appid> <date>")?;
    if args.next().is_some() {
        return Err("unexpected extra argument".into());
    }
    let key = ApiKey::financial(std::env::var("STEAM_FINANCIAL_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let sales = client
        .partner_financials()
        .get_detailed_sales(DetailedSalesRequest {
            date: &date,
            highwatermark_id: 0,
            include_view_grants: None,
        })
        .await?;
    let wishlist = client
        .partner_financials()
        .get_app_wishlist_reporting(AppWishlistRequest { appid, date: &date })
        .await?;
    // A single page is not a complete daily snapshot. Reconcile every page before advancing cursors.
    let _ = (sales, wishlist);
    println!("Received a sales page and wishlist report; no cursor was persisted.");
    Ok(())
}
