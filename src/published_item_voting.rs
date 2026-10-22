// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Publisher access to Workshop voting summaries through indexed-form POSTs.

use crate::{
    Error, Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::nonzero,
    user_auth::OpaqueResponse,
};

/// Access to both documented published-item voting summary methods.
#[derive(Debug, Clone, Copy)]
pub struct PublishedItemVoting<'a>(pub(crate) &'a SteamClient);

impl PublishedItemVoting<'_> {
    /// Retrieve item voting details for one application. The documented POST is
    /// sent at most once, and the unspecified response remains opaque.
    ///
    /// # HTTP contract
    ///
    /// [ItemVoteSummary](https://partner.steamgames.com/doc/webapi/ISteamPublishedItemVoting#ItemVoteSummary): `POST ISteamPublishedItemVoting/ItemVoteSummary/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/published_item_votes.rs).
    ///
    /// ```no_run
    /// use steam_api_web::{SteamClient, SteamId};
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     steamid: SteamId,
    ///     appid: u32,
    ///     publishedfileids: &[u64],
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client
    ///         .published_item_voting()
    ///         .item_vote_summary(
    ///             steamid,
    ///             appid,
    ///             publishedfileids,
    ///         )
    ///         .await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn item_vote_summary(
        &self,
        steamid: SteamId,
        appid: u32,
        publishedfileids: &[u64],
    ) -> Result<OpaqueResponse> {
        nonzero(appid.into(), "appid must not be zero")?;
        self.0
            .call_opaque(
                Endpoint::post(
                    "ISteamPublishedItemVoting/ItemVoteSummary/v1/",
                    Access::Publisher,
                ),
                vote_params(steamid, publishedfileids)?.with("appid", appid),
            )
            .await
    }

    /// Retrieve the user's votes for the selected items. This endpoint has no
    /// appid parameter; its documented POST is never automatically replayed.
    ///
    /// # HTTP contract
    ///
    /// [UserVoteSummary](https://partner.steamgames.com/doc/webapi/ISteamPublishedItemVoting#UserVoteSummary): `POST ISteamPublishedItemVoting/UserVoteSummary/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/published_item_votes.rs).
    ///
    /// ```no_run
    /// use steam_api_web::{SteamClient, SteamId};
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     steamid: SteamId,
    ///     publishedfileids: &[u64],
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client
    ///         .published_item_voting()
    ///         .user_vote_summary(
    ///             steamid,
    ///             publishedfileids,
    ///         )
    ///         .await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn user_vote_summary(
        &self,
        steamid: SteamId,
        publishedfileids: &[u64],
    ) -> Result<OpaqueResponse> {
        self.0
            .call_opaque(
                Endpoint::post(
                    "ISteamPublishedItemVoting/UserVoteSummary/v1/",
                    Access::Publisher,
                ),
                vote_params(steamid, publishedfileids)?,
            )
            .await
    }
}

fn vote_params(steamid: SteamId, publishedfileids: &[u64]) -> Result<Params> {
    nonzero(steamid.raw(), "steamid must not be zero")?;
    let count = u32::try_from(publishedfileids.len())
        .map_err(|_| Error::InvalidParameter("too many published files"))?;
    nonzero(count.into(), "publishedfileids must not be empty")?;
    let mut params = Params::new().with("steamid", steamid).with("count", count);
    for (index, &publishedfileid) in publishedfileids.iter().enumerate() {
        nonzero(publishedfileid, "publishedfileids must not contain zero")?;
        params.push_indexed("publishedfileid", index, publishedfileid);
    }
    Ok(params)
}
