// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! IUserReviewsService: typed public reviews with filters and cursor pagination.
//!
//! This service replaces the deprecated Store `/appreviews` endpoint. Filters
//! are numeric enums in `input_json`, and successful replies have no `success`
//! marker. The client checks HTTP status and standard Steam error envelopes.

use crate::{
    Error, Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::{integer, nonzero, text},
};
use serde::{Deserialize, Serialize, Serializer};
use serde_json::Value;
use std::collections::BTreeMap;

/// Access to IUserReviewsService methods.
#[derive(Debug, Clone, Copy)]
pub struct UserReviewsService<'a>(pub(crate) &'a SteamClient);

/// Sort order for application reviews.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(i32)]
pub enum ReviewFilter {
    /// Steam's helpfulness ranking; its date window can widen to find results.
    #[default]
    Helpful = 0,
    /// Most recently created first; appropriate for exhaustive pagination.
    Recent = 1,
    /// Most recently updated first; appropriate for exhaustive pagination.
    Updated = 2,
    /// Most funny votes first.
    Funny = 3,
}
impl Serialize for ReviewFilter {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_i32(*self as i32)
    }
}

/// Positive or negative review selection.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(i32)]
pub enum ReviewType {
    /// Both positive and negative reviews.
    #[default]
    All = 0,
    /// Positive reviews.
    Positive = 1,
    /// Negative reviews.
    Negative = 2,
}
impl Serialize for ReviewType {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_i32(*self as i32)
    }
}

/// Selection by the author's purchase channel.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(i32)]
pub enum PurchaseType {
    /// Authors who purchased the app on Steam.
    #[default]
    Steam = 0,
    /// All purchase channels.
    All = 1,
    /// Authors who acquired the app outside Steam.
    NonSteamPurchase = 2,
}
impl Serialize for PurchaseType {
    fn serialize<S: Serializer>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error> {
        serializer.serialize_i32(*self as i32)
    }
}

/// Review filters for one request. Omitted fields retain Steam's defaults.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct AppReviewsOptions<'a> {
    /// Sort order; Steam defaults to Helpful.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<ReviewFilter>,
    /// API language codes. Empty retains English; use `["all"]` for all languages.
    #[serde(skip_serializing_if = "<[&str]>::is_empty")]
    pub languages: &'a [&'a str],
    /// Helpful-filter window in days, 0..=365; zero removes the time limit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub day_range: Option<u32>,
    /// Cursor from the previous page. Omit or use `*` for the first page.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cursor: Option<&'a str>,
    /// Review sentiment; Steam defaults to All.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub review_type: Option<ReviewType>,
    /// Purchase channel; Steam defaults to Steam purchases.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub purchase_type: Option<PurchaseType>,
    /// Page size in 1..=100; Steam defaults to 20.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub num_per_page: Option<u32>,
    /// Exclude off-topic activity; Steam defaults to true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_offtopic_activity: Option<bool>,
    /// API language for the score description; Steam defaults to English.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display_language: Option<&'a str>,
    /// Apply Steam's quality ranking to Helpful results; defaults to true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub use_review_quality: Option<bool>,
    /// Inclusive creation-time lower bound in Unix seconds. Steam ignores it
    /// unless `date_range_end` is also supplied.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_range_start: Option<u32>,
    /// Inclusive creation-time upper bound; requires `date_range_start` to apply.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_range_end: Option<u32>,
    /// Exclude the supplied time range instead of selecting it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub date_range_exclude: Option<bool>,
    /// Minimum author playtime at review creation, in hours.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playtime_min_hours: Option<u32>,
    /// Maximum author playtime at review creation, in hours.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub playtime_max_hours: Option<u32>,
    /// Select authors who played more than half their time on Steam Deck.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub primarily_steam_deck: Option<bool>,
    /// Operating-system filter for the hardware used to write the review.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware_os: Option<&'a str>,
    /// CPU filter for the hardware used to write the review.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware_cpu: Option<&'a str>,
    /// GPU filter for the hardware used to write the review.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware_gpu: Option<&'a str>,
    /// Device-type filter for the hardware used to write the review.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hardware_device_type: Option<&'a str>,
}

/// Counts and the optional first-page score summary.
#[derive(Debug, Clone, Deserialize)]
pub struct ReviewQuerySummary {
    /// Number of reviews in this page.
    pub num_reviews: u32,
    /// Steam's score category; only present for an initial, all-sentiment page.
    pub review_score: Option<u32>,
    /// Localized score description, when a score summary is returned.
    pub review_score_desc: Option<String>,
    /// Positive-review total, when a score summary is returned.
    pub total_positive: Option<u32>,
    /// Negative-review total, when a score summary is returned.
    pub total_negative: Option<u32>,
    /// Review total, when a score summary is returned.
    pub total_reviews: Option<u32>,
    /// Additional summary fields introduced by Steam.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// Public author information. Absent scalar fields remain absent, preserving
/// protobuf default omission and visibility-dependent data without inventing zero.
#[derive(Debug, Clone, Deserialize)]
pub struct ReviewAuthor {
    /// Review author's Steam ID, decoded without floating-point conversion.
    pub steamid: SteamId,
    /// Number of public reviews written by the author.
    pub num_reviews: Option<u32>,
    /// Lifetime playtime in minutes.
    pub playtime_forever: Option<u32>,
    /// Recent playtime in minutes.
    pub playtime_last_two_weeks: Option<u32>,
    /// Playtime at review creation in minutes.
    pub playtime_at_review: Option<u32>,
    /// Steam Deck playtime at review creation in minutes.
    pub deck_playtime_at_review: Option<u32>,
    /// Most recent playtime as Unix seconds.
    pub last_played: Option<u32>,
    /// Additional author fields introduced by Steam.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// A public application review.
#[derive(Debug, Clone, Deserialize)]
pub struct AppReview {
    /// Unique review ID, accepting numeric or decimal-string wire values.
    #[serde(deserialize_with = "integer")]
    pub recommendationid: u64,
    /// Public information about the author.
    pub author: ReviewAuthor,
    /// API language code of the review.
    pub language: String,
    /// Review text.
    pub review: String,
    /// Creation time as Unix seconds.
    pub timestamp_created: u32,
    /// Last update time as Unix seconds.
    pub timestamp_updated: u32,
    /// Whether the author recommends the app.
    pub voted_up: bool,
    /// Helpful vote count.
    pub votes_up: u32,
    /// Funny vote count.
    pub votes_funny: u32,
    /// Numeric helpfulness score.
    pub weighted_vote_score: f64,
    /// Number of comments.
    pub comment_count: u32,
    /// Whether the author purchased the app on Steam.
    pub steam_purchase: bool,
    /// Whether the author disclosed receiving the app for free.
    pub received_for_free: bool,
    /// Whether the app was in Early Access when the review was written.
    pub written_during_early_access: bool,
    /// Developer reply, if supplied.
    pub developer_response: Option<String>,
    /// Time of the developer reply, if supplied.
    pub timestamp_dev_responded: Option<u32>,
    /// Whether the author primarily used Steam Deck at review creation.
    pub primarily_steam_deck: bool,
    /// Whether the app was refunded to the author.
    pub refunded: bool,
    /// Additional review fields introduced by Steam.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

/// A single page of public reviews. Use Recent or Updated when exhausting pages;
/// the Helpful filter widens its date range and does not guarantee an empty page.
#[derive(Debug, Clone, Deserialize)]
pub struct AppReviewsPage {
    /// Counts and optional score information for this query.
    pub query_summary: ReviewQuerySummary,
    /// Reviews in this page; protobuf may omit an empty repeated field.
    #[serde(default)]
    pub reviews: Vec<AppReview>,
    /// Continuation cursor. Its characters must be retained unchanged.
    pub cursor: String,
    /// Helpful-filter window actually searched; zero means unbounded.
    pub day_range_used: Option<u32>,
    /// Total reviews matching the query.
    pub total_matching: u32,
    /// Additional response fields introduced by Steam.
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

impl UserReviewsService<'_> {
    /// Read one review page through the current Web API service. An associated
    /// publisher key increases the rate limit, but anonymous requests work too.
    /// Anonymous results may be cached by Steam for ten minutes.
    ///
    /// # HTTP contract
    ///
    /// [GetAppReviews](https://partner.steamgames.com/doc/webapi/IUserReviewsService#GetAppReviews): `GET IUserReviewsService/GetAppReviews/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/app_reviews.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    /// use steam_api_web::reviews::AppReviewsOptions;
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     appid: u32,
    ///     options: AppReviewsOptions<'_>,
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client.reviews().get_app_reviews(appid, options).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_app_reviews(
        &self,
        appid: u32,
        options: AppReviewsOptions<'_>,
    ) -> Result<AppReviewsPage> {
        nonzero(appid.into(), "appid must not be zero")?;
        options.validate()?;
        #[derive(Serialize)]
        struct Request<'a> {
            appid: u32,
            #[serde(flatten)]
            options: AppReviewsOptions<'a>,
        }
        self.0
            .call(
                Endpoint::get("IUserReviewsService/GetAppReviews/v1/", Access::Anonymous),
                Params::json(&Request { appid, options })?,
                &["response"],
            )
            .await
    }
}

impl AppReviewsOptions<'_> {
    fn validate(self) -> Result<()> {
        if self.day_range.is_some_and(|value| value > 365) {
            return Err(Error::InvalidParameter("day_range must be in 0..=365"));
        }
        if self
            .num_per_page
            .is_some_and(|value| !(1..=100).contains(&value))
        {
            return Err(Error::InvalidParameter("num_per_page must be in 1..=100"));
        }
        if matches!((self.date_range_start, self.date_range_end), (Some(start), Some(end)) if start > end)
        {
            return Err(Error::InvalidParameter(
                "date_range_start must not exceed date_range_end",
            ));
        }
        if matches!((self.playtime_min_hours, self.playtime_max_hours), (Some(min), Some(max)) if min > max)
        {
            return Err(Error::InvalidParameter(
                "playtime_min_hours must not exceed playtime_max_hours",
            ));
        }
        for language in self.languages {
            text(language, "review language must not be empty or contain NUL")?;
        }
        for value in [
            self.cursor,
            self.display_language,
            self.hardware_os,
            self.hardware_cpu,
            self.hardware_gpu,
            self.hardware_device_type,
        ]
        .into_iter()
        .flatten()
        {
            text(value, "review filter text must not be empty or contain NUL")?;
        }
        Ok(())
    }
}
