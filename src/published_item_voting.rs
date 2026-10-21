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
