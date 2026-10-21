// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Financial API Group reports from IPartnerFinancialsService.
//!
//! A dedicated Financial API Group key grants account-wide report access. It is
//! separate from a user or publisher key; use a separate client and keep the key
//! on a trusted backend. All three methods are read-only GETs using `input_json`
//! and the documented `v001` paths. Steam enforces account and view-grant rights.
//! Debug output hides financial reports and request dates/cursors.
//!
//! Sales dates use Pacific Time; wishlist dates use GMT. Their exact date strings
//! are retained without timezone conversion. Changed-date and per-date cursors
//! are independent: start each changed date at zero, replace its complete local
//! snapshot, and commit the outer cursor only after all changed dates succeed.
//! No method automatically polls, downloads, persists, or fetches another page.

use crate::{
    Error, Result, SteamClient,
    request::{Access, Endpoint, Params},
    types::{integer, nonzero, optional_integer, valid_calendar_date},
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{collections::BTreeMap, fmt};

/// Read-only access to the three documented financial-reporting methods.
#[derive(Debug, Clone, Copy)]
pub struct PartnerFinancialsService<'a>(pub(crate) &'a SteamClient);

/// A partner-wide change cursor, independent of the per-date sales cursor.
#[derive(Clone, Copy, Serialize)]
pub struct ChangedDatesRequest {
    /// Zero starts a complete scan; later calls use the prior result cursor.
    pub highwatermark: u64,
    /// Include dates exposed through view grants when explicitly requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_view_grants: Option<bool>,
}

/// One page of financial aggregates for one Pacific Time calculation date.
#[derive(Clone, Copy, Serialize)]
pub struct DetailedSalesRequest<'a> {
    /// YYYY-MM-DD, YYYY/MM/DD, or YYYYMMDD, passed unchanged to Steam.
    pub date: &'a str,
    /// Start at zero for each changed date; then use that date's returned max_id.
    pub highwatermark_id: u64,
    /// Include sales exposed through view grants when explicitly requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_view_grants: Option<bool>,
}

/// One application and one GMT-bounded wishlist reporting date.
#[derive(Clone, Copy, Serialize)]
pub struct AppWishlistRequest<'a> {
    /// Application whose wishlist activity is requested.
    pub appid: u32,
    /// YYYY-MM-DD, YYYY/MM/DD, or YYYYMMDD, without timezone conversion.
    pub date: &'a str,
}

/// Dates whose complete sales snapshot must be replaced after recalculation.
#[derive(Clone, Deserialize)]
pub struct ChangedDatesResponse {
    /// Pacific Time date strings. A previously queried date can appear again.
    #[serde(default)]
    pub dates: Vec<String>,
    /// Exact decimal string supplied by Steam. Parse explicitly to uint64 before
    /// constructing the next request; never use the per-date max_id here.
    pub result_highwatermark: String,
    /// Unknown response metadata, retained without an inferred schema.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// One page of daily sales; empty rows alone do not prove completion.
#[derive(Clone, Deserialize)]
pub struct DetailedSalesResponse {
    /// Relevant fields vary between package sales, microtransactions and keys.
    #[serde(default)]
    pub results: Vec<DetailedSalesResult>,
    /// Documented signed int64 cursor, accepting Steam's decimal-string wire
    /// representation too. Completion requires equality with the input cursor.
    /// Convert explicitly to uint64 before a later request; never cast negatives.
    #[serde(deserialize_with = "integer")]
    pub max_id: i64,
    /// Lookup arrays such as package_info, app_info and discount_info, plus any
    /// future metadata. Values are kept in their original JSON representation.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Financial aggregate with conditional fields and exact decimal money strings.
/// Unknown categories, revenue-share tiers and extension fields are preserved.
#[derive(Clone, Deserialize)]
pub struct DetailedSalesResult {
    /// Partner that generated this aggregate, accepting integer or decimal text.
    #[serde(deserialize_with = "integer")]
    pub partnerid: i32,
    /// Calculation date in the same format as the request, in Pacific Time.
    pub date: String,
    /// Steam category such as Package or MicroTxn; future categories are kept.
    pub line_item_type: String,
    /// Country of purchase, preserving Steam's exact code.
    pub country_code: String,
    /// Sold or activated package ID when this is a package aggregate.
    #[serde(default, deserialize_with = "optional_integer")]
    pub packageid: Option<u32>,
    /// Containing bundle ID, if applicable.
    #[serde(default, deserialize_with = "optional_integer")]
    pub bundleid: Option<u32>,
    /// Owning application for an in-game item aggregate.
    #[serde(default, deserialize_with = "optional_integer")]
    pub appid: Option<u32>,
    /// Signed game-item identifier for a microtransaction.
    #[serde(default, deserialize_with = "optional_integer")]
    pub game_item_id: Option<i32>,
    /// Package redemption channel, such as Steam or Retail.
    pub package_sale_type: Option<String>,
    /// Request that generated an activated key, when applicable.
    #[serde(default, deserialize_with = "optional_integer")]
    pub key_request_id: Option<u32>,
    /// Platform attributed to a package sale or key activation.
    pub platform: Option<String>,
    /// Base price in local-currency cents, with no floating-point conversion.
    #[serde(default, deserialize_with = "optional_integer")]
    pub base_price: Option<u64>,
    /// Final price in local-currency cents, with no floating-point conversion.
    #[serde(default, deserialize_with = "optional_integer")]
    pub sale_price: Option<u64>,
    /// Original transaction currency, when available.
    pub currency: Option<String>,
    /// Signed ID for the combination of discounts, when applicable.
    #[serde(default, deserialize_with = "optional_integer")]
    pub combined_discount_id: Option<i32>,
    /// Units sold, preserving signed integer values.
    #[serde(default, deserialize_with = "optional_integer")]
    pub gross_units_sold: Option<i32>,
    /// Returned units can be negative in Steam's response.
    #[serde(default, deserialize_with = "optional_integer")]
    pub gross_units_returned: Option<i32>,
    /// Gross sales in USD dollars, retaining the exact four-decimal text.
    pub gross_sales_usd: Option<String>,
    /// Gross returns in USD dollars, retaining sign and decimal scale.
    pub gross_returns_usd: Option<String>,
    /// Net taxes in USD dollars, retaining sign and decimal scale.
    pub net_tax_usd: Option<String>,
    /// Activated retail keys, if this is a key-activation aggregate.
    #[serde(default, deserialize_with = "optional_integer")]
    pub gross_units_activated: Option<i32>,
    /// Partner that granted access to this row, when applicable.
    #[serde(default, deserialize_with = "optional_integer")]
    pub view_grant_partnerid: Option<i32>,
    /// Units sold less units returned, retaining signed integer values.
    #[serde(default, deserialize_with = "optional_integer")]
    pub net_units_sold: Option<i32>,
    /// Sales less returns and taxes in exact USD decimal text.
    pub net_sales_usd: Option<String>,
    /// Average microtransaction sale price in exact USD decimal text.
    pub avg_sale_price_usd: Option<String>,
    /// Primary application for additional revenue share; zero is meaningful.
    #[serde(default, deserialize_with = "optional_integer")]
    pub primary_appid: Option<i32>,
    /// Raw signed revenue-share tier, preserving future values.
    #[serde(default, deserialize_with = "optional_integer")]
    pub additional_revenue_share_tier: Option<i32>,
    /// Fields not in the documented sales-result schema.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Wishlist activity. An out-of-range date can return only app_min_date.
#[derive(Clone, Deserialize)]
pub struct AppWishlistResponse {
    /// App ID, absent when the requested date is outside the available range.
    pub appid: Option<u32>,
    /// Report date in GMT, absent for an out-of-range request.
    pub date: Option<String>,
    /// Overall activity, absent for an out-of-range request.
    pub wishlist_summary: Option<WishlistSummary>,
    /// Country breakdown, absent or empty when no breakdown is returned.
    #[serde(default)]
    pub country_summary: Vec<CountryWishlistSummary>,
    /// Language breakdown, preserving Steam's numeric language identifiers.
    #[serde(default)]
    pub language_summary: Vec<LanguageWishlistSummary>,
    /// Earliest available report date, including in out-of-range responses.
    pub app_min_date: String,
    /// Other metadata, including example-only fields such as time_generated.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Exact uint32 wishlist counts; no conversion to percentages or money occurs.
#[derive(Clone, Deserialize)]
pub struct WishlistSummary {
    /// Wishlist additions.
    pub wishlist_adds: u32,
    /// Wishlist deletions.
    pub wishlist_deletes: u32,
    /// Purchases from wishlists.
    pub wishlist_purchases: u32,
    /// Gifts from wishlists.
    pub wishlist_gifts: u32,
    /// Additions by users browsing for Windows games.
    pub wishlist_adds_windows: u32,
    /// Additions by users browsing for Mac games.
    pub wishlist_adds_mac: u32,
    /// Additions by users browsing for Linux games.
    pub wishlist_adds_linux: u32,
    /// Additional future counters, retained unchanged.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Wishlist activity for one country, with Steam's original descriptions.
#[derive(Clone, Deserialize)]
pub struct CountryWishlistSummary {
    /// Country code supplied by Steam.
    pub country_code: String,
    /// Country display name supplied by Steam.
    pub country_name: String,
    /// Geographic region supplied by Steam.
    pub region: String,
    /// Activity counters for this country.
    pub summary_actions: WishlistSummary,
    /// Other country metadata.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Wishlist activity for one language; IDs are not restricted to a fixed enum.
#[derive(Clone, Deserialize)]
pub struct LanguageWishlistSummary {
    /// Steam language identifier; zero is valid.
    pub language: u32,
    /// Steam's original language name.
    pub language_name: String,
    /// Activity counters for this language.
    pub summary_actions: WishlistSummary,
    /// Other language metadata.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

macro_rules! redacted_debug {
    ($($name:ty),+ $(,)?) => {$(
        impl fmt::Debug for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(concat!(stringify!($name), "([REDACTED])"))
            }
        }
    )+};
}

redacted_debug!(
    ChangedDatesRequest,
    DetailedSalesRequest<'_>,
    AppWishlistRequest<'_>,
    ChangedDatesResponse,
    DetailedSalesResponse,
    DetailedSalesResult,
    AppWishlistResponse,
    WishlistSummary,
    CountryWishlistSummary,
    LanguageWishlistSummary,
);

impl PartnerFinancialsService<'_> {
    /// Read one account-wide change set. Previously seen dates must be queried
    /// from their beginning again because rows can be added, removed or changed.
    /// Persist result_highwatermark only after all returned dates are processed.
    pub async fn get_changed_dates_for_partner(
        &self,
        request: ChangedDatesRequest,
    ) -> Result<ChangedDatesResponse> {
        self.0
            .call(
                Endpoint::get(
                    "IPartnerFinancialsService/GetChangedDatesForPartner/v001/",
                    Access::Financial,
                ),
                Params::json(&request)?,
                &["response"],
            )
            .await
    }

    /// Read one page for a Pacific Time date. Continue only when max_id differs
    /// from highwatermark_id; even a short or empty page is not a stop condition.
    /// Decimal USD amounts remain strings and local-currency cents remain uint64.
    pub async fn get_detailed_sales(
        &self,
        request: DetailedSalesRequest<'_>,
    ) -> Result<DetailedSalesResponse> {
        validate_date(request.date)?;
        self.0
            .call(
                Endpoint::get(
                    "IPartnerFinancialsService/GetDetailedSales/v001/",
                    Access::Financial,
                ),
                Params::json(&request)?,
                &["response"],
            )
            .await
    }

    /// Read one GMT-bounded wishlist date. Steam can return only app_min_date
    /// outside the available range. This method never enumerates account apps;
    /// that operation uses a separate normal publisher key and client.
    pub async fn get_app_wishlist_reporting(
        &self,
        request: AppWishlistRequest<'_>,
    ) -> Result<AppWishlistResponse> {
        nonzero(request.appid.into(), "appid must not be zero")?;
        validate_date(request.date)?;
        self.0
            .call(
                Endpoint::get(
                    "IPartnerFinancialsService/GetAppWishlistReporting/v001/",
                    Access::Financial,
                ),
                Params::json(&request)?,
                &["response"],
            )
            .await
    }
}

fn validate_date(date: &str) -> Result<()> {
    let bytes = date.as_bytes();
    let (year, month, day) = match bytes {
        [a, b, c, d, e, f, g, h] => ([*a, *b, *c, *d], [*e, *f], [*g, *h]),
        [a, b, c, d, separator, e, f, same, g, h]
            if matches!(separator, b'-' | b'/') && separator == same =>
        {
            ([*a, *b, *c, *d], [*e, *f], [*g, *h])
        }
        _ => {
            return Err(Error::InvalidParameter(
                "date must use a documented date format",
            ));
        }
    };
    fn digits(bytes: &[u8]) -> Option<u32> {
        bytes.iter().try_fold(0, |value, byte| {
            byte.is_ascii_digit()
                .then(|| value * 10 + u32::from(byte - b'0'))
        })
    }
    let (Some(year), Some(month), Some(day)) = (digits(&year), digits(&month), digits(&day)) else {
        return Err(Error::InvalidParameter(
            "date must use ASCII decimal digits",
        ));
    };
    if !valid_calendar_date(year, month, day) {
        return Err(Error::InvalidParameter(
            "date must be a valid calendar date",
        ));
    }
    Ok(())
}
