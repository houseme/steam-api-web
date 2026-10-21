// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! IStoreService: filtered, incremental Steam Store catalog discovery.

use crate::{
    Error, Result, SteamClient,
    client::{Access, Endpoint},
    player::input_json,
    types::text,
    user_auth::OpaqueResponse,
};
use serde::Serialize;

/// Access to IStoreService methods.
#[derive(Debug, Clone, Copy)]
pub struct StoreService<'a>(pub(crate) &'a SteamClient);

/// Filters and continuation for a single catalog page. Omitted flags retain
/// Steam's defaults, which include games and exclude other item types.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct AppListOptions<'a> {
    /// Return items modified since this Unix timestamp.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub if_modified_since: Option<u32>,
    /// Include only items with a description in this API language.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub have_description_language: Option<&'a str>,
    /// Include games, overriding Steam's default of true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_games: Option<bool>,
    /// Include downloadable content.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_dlc: Option<bool>,
    /// Include software.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_software: Option<bool>,
    /// Include videos and series.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_videos: Option<bool>,
    /// Include hardware.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub include_hardware: Option<bool>,
    /// Last app ID from the preceding page; omit on the initial request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_appid: Option<u32>,
    /// Page size in 1..=50,000; omitted uses Steam's default of 10,000.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_results: Option<u32>,
}

impl StoreService<'_> {
    /// Read one catalog page using `input_json` and any configured API key.
    /// Pass the last returned app ID as `last_appid` for the next request. The
    /// reference documents item metadata but no complete response envelope or
    /// termination flag, so the original JSON is preserved without inference.
    pub async fn get_app_list(&self, options: AppListOptions<'_>) -> Result<OpaqueResponse> {
        if options
            .max_results
            .is_some_and(|value| !(1..=50_000).contains(&value))
        {
            return Err(Error::InvalidParameter("max_results must be in 1..=50000"));
        }
        if let Some(language) = options.have_description_language {
            text(
                language,
                "description language must not be empty or contain NUL",
            )?;
        }
        self.0
            .call(
                Endpoint::get("IStoreService/GetAppList/v1/", Access::User),
                input_json(&options)?,
                &[],
            )
            .await
    }
}
