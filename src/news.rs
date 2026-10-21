// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! ISteamNews: public and publisher-only application news.

use crate::{
    Result, SteamClient,
    request::{Access, Endpoint, Params},
    types::{nonzero, text},
    user_auth::OpaqueResponse,
};

/// Access to ISteamNews methods.
#[derive(Debug, Clone, Copy)]
pub struct News<'a>(pub(crate) &'a SteamClient);

/// Optional news filters. Omitted values retain the server's defaults.
#[derive(Debug, Clone, Copy, Default)]
pub struct NewsOptions<'a> {
    /// Maximum content length; zero asks for the complete article.
    pub maxlength: Option<u32>,
    /// Include posts earlier than this Unix timestamp.
    pub enddate: Option<u32>,
    /// Number of posts; omitted defaults to 20 on Steam.
    pub count: Option<u32>,
    /// Comma-separated feed names.
    pub feeds: Option<&'a str>,
}

impl News<'_> {
    /// Read public application news through v2. The reference omits the response
    /// schema, so this method preserves the entire original JSON document.
    pub async fn get_news_for_app(
        &self,
        appid: u32,
        options: NewsOptions<'_>,
    ) -> Result<OpaqueResponse> {
        self.0
            .call_opaque(
                Endpoint::get("ISteamNews/GetNewsForApp/v2/", Access::Anonymous),
                options.params(appid)?,
            )
            .await
    }

    /// Read news including unreleased apps. Requires a publisher key owning the
    /// app and must be called from a secure server. The full JSON is retained.
    pub async fn get_news_for_app_authed(
        &self,
        appid: u32,
        options: NewsOptions<'_>,
    ) -> Result<OpaqueResponse> {
        self.0
            .call_opaque(
                Endpoint::get("ISteamNews/GetNewsForAppAuthed/v2/", Access::Publisher),
                options.params(appid)?,
            )
            .await
    }
}

impl NewsOptions<'_> {
    fn params(self, appid: u32) -> Result<Params> {
        nonzero(appid.into(), "appid must not be zero")?;
        let mut params = Params::new().with("appid", appid);
        if let Some(value) = self.maxlength {
            params.push("maxlength", value);
        }
        if let Some(value) = self.enddate {
            params.push("enddate", value);
        }
        if let Some(value) = self.count {
            params.push("count", value);
        }
        if let Some(value) = self.feeds {
            text(value, "feeds must not be empty or contain NUL")?;
            params.push("feeds", value);
        }
        Ok(params)
    }
}
