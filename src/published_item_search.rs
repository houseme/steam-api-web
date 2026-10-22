// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Publisher Workshop search using the documented indexed-form POST methods.
//! Response structures are unspecified and therefore retained as opaque JSON.

use crate::{
    Error, Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::{nonzero, text},
    user_auth::OpaqueResponse,
};

/// Access to all four documented legacy published-item search methods.
#[derive(Debug, Clone, Copy)]
pub struct PublishedItemSearch<'a>(pub(crate) &'a SteamClient);

/// Shared search filters. Empty tag lists produce a count of zero.
#[derive(Debug, Clone, Copy, Default)]
pub struct PublishedItemSearchOptions<'a> {
    /// Tags to match; indexed field counts are derived from this slice.
    pub tags: &'a [&'a str],
    /// User-specific tags; indexed counts are derived from this slice.
    pub user_tags: &'a [&'a str],
    /// Whether Steam should include private items visible to an app administrator.
    pub hasappadminaccess: Option<bool>,
    /// Optional matching file type; the wire parameter is case-sensitive `fileType`.
    pub file_type: Option<u32>,
}

/// One ranked result page, using Steam's zero-based start offset.
#[derive(Debug, Clone, Copy)]
pub struct PublishedItemSearchRequest<'a> {
    /// User on whose behalf the publisher queries.
    pub steamid: SteamId,
    /// Application whose items are searched.
    pub appid: u32,
    /// Zero-based starting result index.
    pub startidx: u32,
    /// Requested number of results; the reference gives no upper bound.
    pub count: u32,
    /// Optional filters and administrator visibility.
    pub options: PublishedItemSearchOptions<'a>,
}

impl PublishedItemSearch<'_> {
    /// Search by publication date. The documented POST is sent at most once.
    ///
    /// # HTTP contract
    ///
    /// [RankedByPublicationOrder](https://partner.steamgames.com/doc/webapi/ISteamPublishedItemSearch#RankedByPublicationOrder): `POST ISteamPublishedItemSearch/RankedByPublicationOrder/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/published_item_search.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    /// use steam_api_web::published_item_search::PublishedItemSearchRequest;
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     request: PublishedItemSearchRequest<'_>,
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client.published_item_search().ranked_by_publication_order(request).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn ranked_by_publication_order(
        &self,
        request: PublishedItemSearchRequest<'_>,
    ) -> Result<OpaqueResponse> {
        self.ranked(
            "ISteamPublishedItemSearch/RankedByPublicationOrder/v1/",
            request,
            None,
        )
        .await
    }

    /// Search by recent votes, optionally selecting a one-to-seven-day window.
    /// Omission retains Steam's default; this POST is never automatically replayed.
    ///
    /// # HTTP contract
    ///
    /// [RankedByTrend](https://partner.steamgames.com/doc/webapi/ISteamPublishedItemSearch#RankedByTrend): `POST ISteamPublishedItemSearch/RankedByTrend/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/published_item_search.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    /// use steam_api_web::published_item_search::PublishedItemSearchRequest;
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     request: PublishedItemSearchRequest<'_>,
    ///     days: Option<u32>,
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client.published_item_search().ranked_by_trend(request, days).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn ranked_by_trend(
        &self,
        request: PublishedItemSearchRequest<'_>,
        days: Option<u32>,
    ) -> Result<OpaqueResponse> {
        if days.is_some_and(|days| !(1..=7).contains(&days)) {
            return Err(Error::InvalidParameter("days must be between 1 and 7"));
        }
        self.ranked("ISteamPublishedItemSearch/RankedByTrend/v1/", request, days)
            .await
    }

    /// Search by votes. The documented POST is sent at most once.
    ///
    /// # HTTP contract
    ///
    /// [RankedByVote](https://partner.steamgames.com/doc/webapi/ISteamPublishedItemSearch#RankedByVote): `POST ISteamPublishedItemSearch/RankedByVote/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/published_item_search.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    /// use steam_api_web::published_item_search::PublishedItemSearchRequest;
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     request: PublishedItemSearchRequest<'_>,
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client.published_item_search().ranked_by_vote(request).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn ranked_by_vote(
        &self,
        request: PublishedItemSearchRequest<'_>,
    ) -> Result<OpaqueResponse> {
        self.ranked("ISteamPublishedItemSearch/RankedByVote/v1/", request, None)
            .await
    }

    /// Summarize the result set. This endpoint uniquely documents a 64-bit app ID,
    /// which is preserved without narrowing to the ranked endpoints' 32-bit field.
    ///
    /// # HTTP contract
    ///
    /// [ResultSetSummary](https://partner.steamgames.com/doc/webapi/ISteamPublishedItemSearch#ResultSetSummary): `POST ISteamPublishedItemSearch/ResultSetSummary/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/published_item_search.rs).
    ///
    /// ```no_run
    /// use steam_api_web::{SteamClient, SteamId};
    /// use steam_api_web::published_item_search::PublishedItemSearchOptions;
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     steamid: SteamId,
    ///     appid: u64,
    ///     options: PublishedItemSearchOptions<'_>,
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client
    ///         .published_item_search()
    ///         .result_set_summary(
    ///             steamid,
    ///             appid,
    ///             options,
    ///         )
    ///         .await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn result_set_summary(
        &self,
        steamid: SteamId,
        appid: u64,
        options: PublishedItemSearchOptions<'_>,
    ) -> Result<OpaqueResponse> {
        nonzero(steamid.raw(), "steamid must not be zero")?;
        nonzero(appid, "appid must not be zero")?;
        let mut params = Params::new().with("steamid", steamid).with("appid", appid);
        filters(&mut params, options)?;
        self.0
            .call_opaque(
                Endpoint::post(
                    "ISteamPublishedItemSearch/ResultSetSummary/v1/",
                    Access::Publisher,
                ),
                params,
            )
            .await
    }

    async fn ranked(
        &self,
        path: &'static str,
        request: PublishedItemSearchRequest<'_>,
        days: Option<u32>,
    ) -> Result<OpaqueResponse> {
        nonzero(request.steamid.raw(), "steamid must not be zero")?;
        nonzero(request.appid.into(), "appid must not be zero")?;
        nonzero(request.count.into(), "count must not be zero")?;
        let mut params = Params::new()
            .with("steamid", request.steamid)
            .with("appid", request.appid)
            .with("startidx", request.startidx)
            .with("count", request.count);
        filters(&mut params, request.options)?;
        if let Some(days) = days {
            params.push("days", days);
        }
        self.0
            .call_opaque(Endpoint::post(path, Access::Publisher), params)
            .await
    }
}

fn filters(params: &mut Params, options: PublishedItemSearchOptions<'_>) -> Result<()> {
    let tags =
        u32::try_from(options.tags.len()).map_err(|_| Error::InvalidParameter("too many tags"))?;
    let user_tags = u32::try_from(options.user_tags.len())
        .map_err(|_| Error::InvalidParameter("too many user tags"))?;
    params.push("tagcount", tags);
    params.push("usertagcount", user_tags);
    for (prefix, values) in [("tag", options.tags), ("usertag", options.user_tags)] {
        for (index, value) in values.iter().enumerate() {
            text(value, "tags must be nonempty and contain no NUL")?;
            params.push_indexed(prefix, index, value);
        }
    }
    if let Some(value) = options.hasappadminaccess {
        params.push("hasappadminaccess", value);
    }
    if let Some(value) = options.file_type {
        params.push("fileType", value);
    }
    Ok(())
}
