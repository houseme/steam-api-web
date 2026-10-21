// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Read one financial change batch without printing dates, cursors or revenue.

use steam_api_web::{ApiKey, SteamClient, partner_financials::ChangedDatesRequest};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let highwatermark = std::env::args()
        .nth(1)
        .map(|value| value.parse::<u64>())
        .transpose()?
        .unwrap_or(0);
    let key = ApiKey::financial(std::env::var("STEAM_FINANCIAL_KEY")?)?;
    let client = SteamClient::builder().api_key(key).build()?;
    let report = client
        .partner_financials()
        .get_changed_dates_for_partner(ChangedDatesRequest {
            highwatermark,
            include_view_grants: None,
        })
        .await?;
    // Process each changed date and persist its complete snapshot before saving
    // report.result_highwatermark. This example never advances or stores cursors.
    println!("Changed dates in this batch: {}", report.dates.len());
    Ok(())
}
