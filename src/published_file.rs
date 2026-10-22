// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Published-file queries and publisher moderation through `IPublishedFileService`.
//!
//! Requests use `input_json`. Valve does not document response envelopes here;
//! results retain the complete JSON and redact their debug representation.

use crate::{
    Error, Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::{nonzero, text},
    user_auth::OpaqueResponse,
};
use serde::Serialize;
use serde_json::Value;

/// Access to all seven documented published-file methods.
#[derive(Debug, Clone, Copy)]
pub struct PublishedFileService<'a>(pub(crate) &'a SteamClient);

/// Select bounded page indexing or lossless deep pagination.
#[derive(Debug, Clone, Copy)]
pub enum QueryFilesPagination<'a> {
    /// One-based page index, at most 1000.
    Page(u32),
    /// Use `"*"` initially, then the unchanged response `next_cursor` value.
    Cursor(&'a str),
}

/// Filters and response controls documented for `QueryFiles`.
/// Empty filters disable their restriction; omitted optional values use Steam's defaults.
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct QueryFilesOptions<'a> {
    /// Optional page size; the reference supplies no numeric maximum.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub numperpage: Option<u32>,
    /// Required tag expression, represented as the documented string field.
    pub requiredtags: &'a str,
    /// Excluded tag expression, represented as the documented string field.
    pub excludedtags: &'a str,
    /// Require every selected tag instead of any selected tag.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub match_all_tags: Option<bool>,
    /// Required flag expression.
    pub required_flags: &'a str,
    /// Excluded flag expression.
    pub omitted_flags: &'a str,
    /// Search text; an empty string leaves the text filter unrestricted.
    pub search_text: &'a str,
    /// Numeric `EPublishedFileInfoMatchingFileType`, distinct from response file types.
    pub filetype: u32,
    /// Match parents of this item; zero leaves this restriction unset.
    pub child_publishedfileid: u64,
    /// Trend window, from one through seven for query type 3.
    pub days: u32,
    /// Restrict trend queries to recently voted items.
    pub include_recent_votes_only: bool,
    /// Optional acceptable cache age, in seconds.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_max_age_seconds: Option<u32>,
    /// Signed language identifier; omitted values use English on Steam.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub language: Option<i32>,
    /// Raw key-value tag message, whose nested schema is absent from the reference.
    /// Supply an object or an array of objects reviewed against your Steam contract.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required_kv_tags: Option<&'a Value>,
    /// Return only the number of matching items.
    pub totalonly: bool,
    /// Return item IDs without full item details.
    pub ids_only: bool,
    /// Include voting information.
    pub return_vote_data: bool,
    /// Include tag information.
    pub return_tags: bool,
    /// Include key-value tag information.
    pub return_kv_tags: bool,
    /// Include preview information.
    pub return_previews: bool,
    /// Include child item references.
    pub return_children: bool,
    /// Prefer short descriptions over the complete file description.
    pub return_short_description: bool,
    /// Include sale-related information where applicable.
    pub return_for_sale_data: bool,
    /// Include developer metadata when explicitly requested.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub return_metadata: Option<bool>,
    /// Number of preceding days for returned playtime statistics.
    pub return_playtime_stats: u32,
}

/// One published-file query with explicit app scope and pagination.
#[derive(Debug, Clone, Copy)]
pub struct QueryFilesRequest<'a> {
    /// Numeric `EPublishedFileQueryType`; unknown future values are preserved.
    pub query_type: u32,
    /// Application used to create the content.
    pub creator_appid: u32,
    /// Application consuming the content.
    pub appid: u32,
    /// Cursor or bounded page selection.
    pub pagination: QueryFilesPagination<'a>,
    /// Query filters and response controls.
    pub options: QueryFilesOptions<'a>,
}

impl PublishedFileService<'_> {
    /// Delete an item through Valve's documented GET mutation.
    /// A publisher key is required; this request is never automatically replayed.
    ///
    /// # HTTP contract
    ///
    /// [Delete](https://partner.steamgames.com/doc/webapi/IPublishedFileService#Delete): `GET IPublishedFileService/Delete/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/workshop_query.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     appid: u32,
    ///     publishedfileid: u64,
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client.published_file().delete(appid, publishedfileid).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn delete(&self, appid: u32, publishedfileid: u64) -> Result<OpaqueResponse> {
        validate_item(appid, publishedfileid)?;
        self.0
            .call_opaque(
                Endpoint::get_once("IPublishedFileService/Delete/v1/", Access::Publisher),
                Params::json(&ItemRequest {
                    publishedfileid,
                    appid,
                })?,
            )
            .await
    }

    /// Search for published files. Inspect the opaque response's `next_cursor`
    /// without changing or decoding it before constructing the next request.
    ///
    /// # HTTP contract
    ///
    /// [QueryFiles](https://partner.steamgames.com/doc/webapi/IPublishedFileService#QueryFiles): `GET IPublishedFileService/QueryFiles/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/workshop_query.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    /// use steam_api_web::published_file::QueryFilesRequest;
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     request: QueryFilesRequest<'_>,
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client.published_file().query_files(request).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn query_files(&self, request: QueryFilesRequest<'_>) -> Result<OpaqueResponse> {
        nonzero(request.appid.into(), "appid must not be zero")?;
        nonzero(
            request.creator_appid.into(),
            "creator_appid must not be zero",
        )?;
        let (page, cursor) = match request.pagination {
            QueryFilesPagination::Page(page) if (1..=1000).contains(&page) => (page, None),
            QueryFilesPagination::Page(_) => {
                return Err(Error::InvalidParameter("page must be between 1 and 1000"));
            }
            QueryFilesPagination::Cursor(cursor) => {
                text(cursor, "cursor must be nonempty and contain no NUL")?;
                // Valve lists both parameters as required and ignores page when a cursor is present.
                (1, Some(cursor))
            }
        };
        let options = request.options;
        for value in [
            options.requiredtags,
            options.excludedtags,
            options.required_flags,
            options.omitted_flags,
            options.search_text,
        ] {
            validate_string(value)?;
        }
        if options.numperpage == Some(0) {
            return Err(Error::InvalidParameter("numperpage must not be zero"));
        }
        if request.query_type == 3 && !(1..=7).contains(&options.days) {
            return Err(Error::InvalidParameter(
                "trend queries require days between 1 and 7",
            ));
        }
        if let Some(value) = options.required_kv_tags {
            validate_message(value)?;
        }
        #[derive(Serialize)]
        struct Request<'a> {
            query_type: u32,
            creator_appid: u32,
            appid: u32,
            page: u32,
            #[serde(skip_serializing_if = "Option::is_none")]
            cursor: Option<&'a str>,
            #[serde(flatten)]
            options: QueryFilesOptions<'a>,
        }
        self.0
            .call_opaque(
                Endpoint::get("IPublishedFileService/QueryFiles/v1/", Access::User),
                Params::json(&Request {
                    query_type: request.query_type,
                    creator_appid: request.creator_appid,
                    appid: request.appid,
                    page,
                    cursor,
                    options,
                })?,
            )
            .await
    }

    /// Replace or clear developer metadata without automatic replay.
    ///
    /// # HTTP contract
    ///
    /// [SetDeveloperMetadata](https://partner.steamgames.com/doc/webapi/IPublishedFileService#SetDeveloperMetadata): `POST IPublishedFileService/SetDeveloperMetadata/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/workshop_query.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     appid: u32,
    ///     publishedfileid: u64,
    ///     metadata: &str,
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client
    ///         .published_file()
    ///         .set_developer_metadata(
    ///             appid,
    ///             publishedfileid,
    ///             metadata,
    ///         )
    ///         .await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn set_developer_metadata(
        &self,
        appid: u32,
        publishedfileid: u64,
        metadata: &str,
    ) -> Result<OpaqueResponse> {
        validate_item(appid, publishedfileid)?;
        validate_string(metadata)?;
        #[derive(Serialize)]
        struct Request<'a> {
            publishedfileid: u64,
            appid: u32,
            metadata: &'a str,
        }
        self.0
            .call_opaque(
                Endpoint::post(
                    "IPublishedFileService/SetDeveloperMetadata/v1/",
                    Access::Publisher,
                ),
                Params::json(&Request {
                    publishedfileid,
                    appid,
                    metadata,
                })?,
            )
            .await
    }

    /// Set a user's app-specific UGC ban; an expiration of zero removes it.
    /// An omitted reason is left for Steam to interpret. Never automatically replayed.
    ///
    /// # HTTP contract
    ///
    /// [UpdateAppUGCBan](https://partner.steamgames.com/doc/webapi/IPublishedFileService#UpdateAppUGCBan): `POST IPublishedFileService/UpdateAppUGCBan/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/workshop_query.rs).
    ///
    /// ```no_run
    /// use steam_api_web::{SteamClient, SteamId};
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     appid: u32,
    ///     steamid: SteamId,
    ///     expiration_time: u32,
    ///     reason: Option<&str>,
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client
    ///         .published_file()
    ///         .update_app_ugc_ban(
    ///             appid,
    ///             steamid,
    ///             expiration_time,
    ///             reason,
    ///         )
    ///         .await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn update_app_ugc_ban(
        &self,
        appid: u32,
        steamid: SteamId,
        expiration_time: u32,
        reason: Option<&str>,
    ) -> Result<OpaqueResponse> {
        nonzero(appid.into(), "appid must not be zero")?;
        nonzero(steamid.raw(), "steamid must not be zero")?;
        if let Some(reason) = reason {
            validate_string(reason)?;
        }
        #[derive(Serialize)]
        struct Request<'a> {
            steamid: SteamId,
            appid: u32,
            expiration_time: u32,
            #[serde(skip_serializing_if = "Option::is_none")]
            reason: Option<&'a str>,
        }
        self.0
            .call_opaque(
                Endpoint::post(
                    "IPublishedFileService/UpdateAppUGCBan/v1/",
                    Access::Publisher,
                ),
                Params::json(&Request {
                    steamid,
                    appid,
                    expiration_time,
                    reason,
                })?,
            )
            .await
    }

    /// Change an item's ban state and administrator-visible reason without replay.
    ///
    /// # HTTP contract
    ///
    /// [UpdateBanStatus](https://partner.steamgames.com/doc/webapi/IPublishedFileService#UpdateBanStatus): `POST IPublishedFileService/UpdateBanStatus/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/workshop_query.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     appid: u32,
    ///     publishedfileid: u64,
    ///     banned: bool,
    ///     reason: &str,
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client
    ///         .published_file()
    ///         .update_ban_status(
    ///             appid,
    ///             publishedfileid,
    ///             banned,
    ///             reason,
    ///         )
    ///         .await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn update_ban_status(
        &self,
        appid: u32,
        publishedfileid: u64,
        banned: bool,
        reason: &str,
    ) -> Result<OpaqueResponse> {
        validate_item(appid, publishedfileid)?;
        validate_string(reason)?;
        #[derive(Serialize)]
        struct Request<'a> {
            publishedfileid: u64,
            appid: u32,
            banned: bool,
            reason: &'a str,
        }
        self.0
            .call_opaque(
                Endpoint::post(
                    "IPublishedFileService/UpdateBanStatus/v1/",
                    Access::Publisher,
                ),
                Params::json(&Request {
                    publishedfileid,
                    appid,
                    banned,
                    reason,
                })?,
            )
            .await
    }

    /// Change whether an item is hidden as incompatible, without automatic replay.
    ///
    /// # HTTP contract
    ///
    /// [UpdateIncompatibleStatus](https://partner.steamgames.com/doc/webapi/IPublishedFileService#UpdateIncompatibleStatus): `POST IPublishedFileService/UpdateIncompatibleStatus/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/workshop_query.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     appid: u32,
    ///     publishedfileid: u64,
    ///     incompatible: bool,
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client
    ///         .published_file()
    ///         .update_incompatible_status(
    ///             appid,
    ///             publishedfileid,
    ///             incompatible,
    ///         )
    ///         .await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn update_incompatible_status(
        &self,
        appid: u32,
        publishedfileid: u64,
        incompatible: bool,
    ) -> Result<OpaqueResponse> {
        validate_item(appid, publishedfileid)?;
        #[derive(Serialize)]
        struct Request {
            publishedfileid: u64,
            appid: u32,
            incompatible: bool,
        }
        self.0
            .call_opaque(
                Endpoint::post(
                    "IPublishedFileService/UpdateIncompatibleStatus/v1/",
                    Access::Publisher,
                ),
                Params::json(&Request {
                    publishedfileid,
                    appid,
                    incompatible,
                })?,
            )
            .await
    }

    /// Add and remove tags; an empty list leaves that operation unset.
    /// Existing tags outside `remove_tags` are preserved by Steam. Never replayed.
    ///
    /// # HTTP contract
    ///
    /// [UpdateTags](https://partner.steamgames.com/doc/webapi/IPublishedFileService#UpdateTags): `POST IPublishedFileService/UpdateTags/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/workshop_query.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     appid: u32,
    ///     publishedfileid: u64,
    ///     add_tags: &[&str],
    ///     remove_tags: &[&str],
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client
    ///         .published_file()
    ///         .update_tags(
    ///             appid,
    ///             publishedfileid,
    ///             add_tags,
    ///             remove_tags,
    ///         )
    ///         .await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn update_tags(
        &self,
        appid: u32,
        publishedfileid: u64,
        add_tags: &[&str],
        remove_tags: &[&str],
    ) -> Result<OpaqueResponse> {
        validate_item(appid, publishedfileid)?;
        for tag in add_tags.iter().chain(remove_tags) {
            text(tag, "tags must be nonempty and contain no NUL")?;
        }
        #[derive(Serialize)]
        struct Request<'a> {
            publishedfileid: u64,
            appid: u32,
            #[serde(skip_serializing_if = "<[&str]>::is_empty")]
            add_tags: &'a [&'a str],
            #[serde(skip_serializing_if = "<[&str]>::is_empty")]
            remove_tags: &'a [&'a str],
        }
        self.0
            .call_opaque(
                Endpoint::post("IPublishedFileService/UpdateTags/v1/", Access::Publisher),
                Params::json(&Request {
                    publishedfileid,
                    appid,
                    add_tags,
                    remove_tags,
                })?,
            )
            .await
    }
}

#[derive(Serialize)]
struct ItemRequest {
    publishedfileid: u64,
    appid: u32,
}

pub(crate) fn validate_item(appid: u32, publishedfileid: u64) -> Result<()> {
    nonzero(appid.into(), "appid must not be zero")?;
    nonzero(publishedfileid, "publishedfileid must not be zero")
}

pub(crate) fn validate_string(value: &str) -> Result<()> {
    if value.contains('\0') {
        return Err(Error::InvalidParameter("text must not contain NUL"));
    }
    Ok(())
}

pub(crate) fn validate_message(value: &Value) -> Result<()> {
    if value.is_object()
        || value
            .as_array()
            .is_some_and(|values| values.iter().all(Value::is_object))
    {
        return Ok(());
    }
    Err(Error::InvalidParameter(
        "undocumented message must be an object or an array of objects",
    ))
}
