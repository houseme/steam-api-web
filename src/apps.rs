// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! ISteamApps: application administration, public catalogs, and server discovery.
//!
//! Publisher methods require an API key belonging to the application owner and
//! must run on trusted servers. Undocumented response schemas remain opaque.

use crate::{
    Error, Result, SteamClient, SteamId,
    request::{Access, CommaSeparated, Endpoint, FormValue, Params},
    types::{nonzero, text},
    user_auth::OpaqueResponse,
};
use serde::Deserialize;
use std::net::{IpAddr, SocketAddr};

/// Access to all documented ISteamApps methods.
#[derive(Debug, Clone, Copy)]
pub struct Apps<'a>(pub(crate) &'a SteamClient);

/// An application type accepted by the publisher application list filter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppType {
    /// A game.
    Game,
    /// A non-game application.
    Application,
    /// A tool, including dedicated servers.
    Tool,
    /// A demonstration application.
    Demo,
    /// Downloadable content.
    Dlc,
    /// A soundtrack or music application.
    Music,
}

impl AppType {
    fn as_str(self) -> &'static str {
        match self {
            Self::Game => "game",
            Self::Application => "application",
            Self::Tool => "tool",
            Self::Demo => "demo",
            Self::Dlc => "dlc",
            Self::Music => "music",
        }
    }
}

impl FormValue for AppType {
    fn encode(&self, output: &mut String) {
        self.as_str().encode(output);
    }
}

/// Optional time filtering and continuation for one banned-player page.
///
/// Omitted fields retain Steam's defaults. A continuation token requires both
/// time boundaries and must be reused with the same interval as the first page.
/// Responses remain opaque because Valve does not publish their full schema.
#[derive(Debug, Clone, Copy, Default)]
pub struct BannedPlayersOptions<'a> {
    /// Query start as Unix seconds; zero is a valid epoch boundary.
    pub starttime: Option<u32>,
    /// Query end as Unix seconds; must not precede starttime when both are set.
    /// The reference does not specify whether this boundary is inclusive.
    pub endtime: Option<u32>,
    /// Unmodified continuation from the preceding response, when supplied.
    /// Steam uses this only when both time boundaries are present.
    pub continue_token: Option<&'a str>,
}

/// An application associated with a publisher Web API key (v2).
#[derive(Debug, Clone, Deserialize)]
pub struct PartnerApp {
    /// Application ID.
    pub appid: u32,
    /// Steam's application type; unknown future types remain representable.
    pub app_type: String,
    /// Application display name, included by version 2.
    pub app_name: String,
}

/// Parameters for requesting that a build become live on a Steam branch.
///
/// A released application's public branch requires an account with Edit App and
/// Publish permissions. Steam sends that account a mobile confirmation. Supplying
/// an account here does not bypass the confirmation requirement.
#[derive(Debug, Clone, Copy)]
pub struct SetAppBuildLiveRequest<'a> {
    /// Application ID.
    pub appid: u32,
    /// Build ID to make live.
    pub buildid: u32,
    /// Beta branch key; use `public` for the default branch.
    pub betakey: &'a str,
    /// Account that will confirm a released public branch change, when required.
    pub steamid: Option<SteamId>,
    /// Optional description of the build.
    pub description: Option<&'a str>,
}

/// Meaning of Steam's successful build-change HTTP response.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildLiveState {
    /// Steam accepted the request; callers should verify the branch's build.
    RequestAccepted,
    /// HTTP 201 indicates that Steam Mobile confirmation is still required.
    ConfirmationRequired,
}

/// Result of requesting a live build change, preserving the HTTP status.
#[derive(Debug)]
pub struct SetAppBuildLiveResponse {
    /// Successful HTTP status returned by Steam.
    pub http_status: u16,
    /// Whether Steam requires a subsequent mobile confirmation.
    pub state: BuildLiveState,
    /// Original JSON, absent when Steam returns HTTP 201 without a body.
    pub response: Option<OpaqueResponse>,
}

impl Apps<'_> {
    /// Read all beta branches for an owned application (publisher GET v1).
    ///
    /// # HTTP contract
    ///
    /// [GetAppBetas](https://partner.steamgames.com/doc/webapi/ISteamApps#GetAppBetas): `GET ISteamApps/GetAppBetas/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/app_builds.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(client: &SteamClient, appid: u32) -> steam_api_web::Result<()> {
    ///     let response = client.apps().get_app_betas(appid).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_app_betas(&self, appid: u32) -> Result<OpaqueResponse> {
        self.app_read("ISteamApps/GetAppBetas/v1/", appid, Params::new())
            .await
    }

    /// Read an owned application's build history (publisher GET v1).
    /// Omit `count` to use Steam's default of ten; an explicit count must be nonzero.
    ///
    /// # HTTP contract
    ///
    /// [GetAppBuilds](https://partner.steamgames.com/doc/webapi/ISteamApps#GetAppBuilds): `GET ISteamApps/GetAppBuilds/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/app_builds.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     appid: u32,
    ///     count: Option<u32>,
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client.apps().get_app_builds(appid, count).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_app_builds(&self, appid: u32, count: Option<u32>) -> Result<OpaqueResponse> {
        let mut params = Params::new();
        optional_count(&mut params, "count", count)?;
        self.app_read("ISteamApps/GetAppBuilds/v1/", appid, params)
            .await
    }

    /// Read every depot version for an owned application (publisher GET v1).
    ///
    /// # HTTP contract
    ///
    /// [GetAppDepotVersions](https://partner.steamgames.com/doc/webapi/ISteamApps#GetAppDepotVersions): `GET ISteamApps/GetAppDepotVersions/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/app_builds.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(client: &SteamClient, appid: u32) -> steam_api_web::Result<()> {
    ///     let response = client.apps().get_app_depot_versions(appid).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_app_depot_versions(&self, appid: u32) -> Result<OpaqueResponse> {
        self.app_read("ISteamApps/GetAppDepotVersions/v1/", appid, Params::new())
            .await
    }

    /// Read the legacy complete public catalog (anonymous GET v2).
    ///
    /// Valve deprecated this endpoint because it cannot scale to the catalog's
    /// size. Prefer `IStoreService/GetAppList`; the client's response size limit
    /// still applies to this legacy request.
    #[deprecated(note = "Valve recommends IStoreService/GetAppList for catalog pagination")]
    ///
    /// # HTTP contract
    ///
    /// [GetAppList](https://partner.steamgames.com/doc/webapi/ISteamApps#GetAppList): `GET ISteamApps/GetAppList/v2/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/app_builds.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(client: &SteamClient) -> steam_api_web::Result<()> {
    ///     let response = client.apps().get_app_list().await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_app_list(&self) -> Result<OpaqueResponse> {
        self.0
            .call_opaque(
                Endpoint::get("ISteamApps/GetAppList/v2/", Access::Anonymous),
                Params::new(),
            )
            .await
    }

    /// Read applications associated with this publisher key (GET v2).
    /// An empty filter returns all types. Filters are sent as comma-separated text.
    ///
    /// # HTTP contract
    ///
    /// [GetPartnerAppListForWebAPIKey](https://partner.steamgames.com/doc/webapi/ISteamApps#GetPartnerAppListForWebAPIKey): `GET ISteamApps/GetPartnerAppListForWebAPIKey/v2/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/app_builds.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    /// use steam_api_web::apps::AppType;
    ///
    /// async fn example(client: &SteamClient, type_filter: &[AppType]) -> steam_api_web::Result<()> {
    ///     let response = client.apps().get_partner_app_list_for_web_api_key(type_filter).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_partner_app_list_for_web_api_key(
        &self,
        type_filter: &[AppType],
    ) -> Result<Vec<PartnerApp>> {
        if type_filter.len() > 6
            || type_filter
                .iter()
                .enumerate()
                .any(|(index, kind)| type_filter[..index].contains(kind))
        {
            return Err(Error::InvalidParameter(
                "type_filter must contain at most six distinct application types",
            ));
        }
        let params = if type_filter.is_empty() {
            Params::new()
        } else {
            Params::new().with("type_filter", CommaSeparated(type_filter))
        };
        self.0
            .call(
                Endpoint::get(
                    "ISteamApps/GetPartnerAppListForWebAPIKey/v2/",
                    Access::Publisher,
                ),
                params,
                &["applist", "apps", "app"],
            )
            .await
    }

    /// Read an application's banned players (publisher GET v1).
    ///
    /// # HTTP contract
    ///
    /// [GetPlayersBanned](https://partner.steamgames.com/doc/webapi/ISteamApps#GetPlayersBanned): `GET ISteamApps/GetPlayersBanned/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/app_builds.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(client: &SteamClient, appid: u32) -> steam_api_web::Result<()> {
    ///     let response = client.apps().get_players_banned(appid).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_players_banned(&self, appid: u32) -> Result<OpaqueResponse> {
        self.get_players_banned_with_options(appid, BannedPlayersOptions::default())
            .await
    }

    /// Read one banned-player page with optional Unix-time filtering and continuation.
    /// The existing [`Self::get_players_banned`] method retains the unfiltered call.
    /// Keep the time interval unchanged when requesting subsequent pages; this
    /// method performs one page operation and never invents a response cursor.
    ///
    /// # HTTP contract
    ///
    /// [GetPlayersBanned](https://partner.steamgames.com/doc/webapi/ISteamApps#GetPlayersBanned):
    /// `GET ISteamApps/GetPlayersBanned/v1/`, requiring a publisher key.
    ///
    /// # Errors
    ///
    /// Returns [`Error::InvalidParameter`] if the time interval is reversed, a
    /// continuation token is blank or contains NUL, or a token lacks either time
    /// boundary. The app ID must be nonzero. Authentication and transport/response
    /// errors follow the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use steam_api_web::{SteamClient, apps::BannedPlayersOptions};
    ///
    /// async fn first_page(client: &SteamClient, appid: u32) -> steam_api_web::Result<()> {
    ///     let response = client.apps().get_players_banned_with_options(appid, BannedPlayersOptions {
    ///         starttime: Some(0), endtime: Some(1_700_000_000), continue_token: None,
    ///     }).await?;
    ///     // Inspect the opaque result for the applicable ban records and continuation.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    ///
    /// See the [standalone paging example](https://github.com/houseme/steam-api-web/blob/main/examples/banned_players.rs).
    pub async fn get_players_banned_with_options(
        &self,
        appid: u32,
        options: BannedPlayersOptions<'_>,
    ) -> Result<OpaqueResponse> {
        if options
            .starttime
            .zip(options.endtime)
            .is_some_and(|(start, end)| start > end)
        {
            return Err(Error::InvalidParameter("starttime must not exceed endtime"));
        }
        let mut params = Params::new();
        if let Some(starttime) = options.starttime {
            params.push("starttime", starttime);
        }
        if let Some(endtime) = options.endtime {
            params.push("endtime", endtime);
        }
        if let Some(token) = options.continue_token {
            if options.starttime.is_none() || options.endtime.is_none() {
                return Err(Error::InvalidParameter(
                    "continue_token requires starttime and endtime",
                ));
            }
            text(token, "continue_token must not be blank or contain NUL")?;
            params.push("continue_token", token);
        }
        self.app_read("ISteamApps/GetPlayersBanned/v1/", appid, params)
            .await
    }

    /// Discover servers using Steam's query filter (publisher GET v1).
    /// The filter is transmitted unchanged; a supplied limit must be nonzero.
    ///
    /// # HTTP contract
    ///
    /// [GetServerList](https://partner.steamgames.com/doc/webapi/ISteamApps#GetServerList): `GET ISteamApps/GetServerList/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/app_builds.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     filter: Option<&str>,
    ///     limit: Option<u32>,
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client.apps().get_server_list(filter, limit).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_server_list(
        &self,
        filter: Option<&str>,
        limit: Option<u32>,
    ) -> Result<OpaqueResponse> {
        let mut params = Params::new();
        if let Some(filter) = filter {
            if filter.contains('\0') {
                return Err(Error::InvalidParameter("filter must not contain NUL"));
            }
            params.push("filter", filter);
        }
        optional_count(&mut params, "limit", limit)?;
        self.0
            .call_opaque(
                Endpoint::get("ISteamApps/GetServerList/v1/", Access::Publisher),
                params,
            )
            .await
    }

    /// Discover servers at an IP address or IP with query port (anonymous GET v1).
    /// IPv6 socket addresses use brackets, for example `[::1]:27015`.
    ///
    /// # HTTP contract
    ///
    /// [GetServersAtAddress](https://partner.steamgames.com/doc/webapi/ISteamApps#GetServersAtAddress): `GET ISteamApps/GetServersAtAddress/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/app_builds.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(client: &SteamClient, addr: &str) -> steam_api_web::Result<()> {
    ///     let response = client.apps().get_servers_at_address(addr).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn get_servers_at_address(&self, addr: &str) -> Result<OpaqueResponse> {
        let valid_ip = addr.parse::<IpAddr>().is_ok();
        let valid_socket = addr
            .parse::<SocketAddr>()
            .is_ok_and(|address| address.port() != 0);
        if !valid_ip && !valid_socket {
            return Err(Error::InvalidParameter(
                "addr must be an IP address or an IP address with a nonzero query port",
            ));
        }
        self.0
            .call_opaque(
                Endpoint::get("ISteamApps/GetServersAtAddress/v1/", Access::Anonymous),
                Params::new().with("addr", addr),
            )
            .await
    }

    /// Request a live build change (publisher POST v2), without automatic replay.
    ///
    /// A successful HTTP response can mean that mobile confirmation is pending;
    /// it does not establish that the build is already live. Steam reports this
    /// condition with HTTP 201. Inspect the returned response and verify the
    /// branch separately before treating a deployment as complete.
    /// The caller must supply `steamid` for released public branches, because
    /// this client cannot infer whether the application has been released.
    ///
    /// # HTTP contract
    ///
    /// [SetAppBuildLive](https://partner.steamgames.com/doc/webapi/ISteamApps#SetAppBuildLive): `POST ISteamApps/SetAppBuildLive/v2/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/app_builds.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    /// use steam_api_web::apps::SetAppBuildLiveRequest;
    ///
    /// async fn example(
    ///     client: &SteamClient,
    ///     request: &SetAppBuildLiveRequest<'_>,
    /// ) -> steam_api_web::Result<()> {
    ///     let response = client.apps().set_app_build_live(request).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn set_app_build_live(
        &self,
        request: &SetAppBuildLiveRequest<'_>,
    ) -> Result<SetAppBuildLiveResponse> {
        nonzero(request.appid.into(), "appid must not be zero")?;
        nonzero(request.buildid.into(), "buildid must not be zero")?;
        text(request.betakey, "betakey must not be empty or contain NUL")?;
        let mut params = Params::new()
            .with("appid", request.appid)
            .with("buildid", request.buildid)
            .with("betakey", request.betakey);
        if let Some(steamid) = request.steamid {
            nonzero(steamid.raw(), "steamid must not be zero")?;
            params.push("steamid", steamid);
        }
        if let Some(description) = request.description {
            if description.contains('\0') {
                return Err(Error::InvalidParameter("description must not contain NUL"));
            }
            params.push("description", description);
        }
        let (http_status, response) = self
            .0
            .call_opaque_with_status(
                Endpoint::post("ISteamApps/SetAppBuildLive/v2/", Access::Publisher)
                    .allow_empty_created(),
                params,
            )
            .await?;
        Ok(SetAppBuildLiveResponse {
            http_status,
            state: if http_status == 201 {
                BuildLiveState::ConfirmationRequired
            } else {
                BuildLiveState::RequestAccepted
            },
            // Only the explicitly allowed empty HTTP 201 produces JSON null.
            response: (!response.as_json().is_null()).then_some(response),
        })
    }

    /// Compare an installed game version with Steam's current version (GET v1).
    /// Version zero is allowed, for example when no local version is known.
    ///
    /// # HTTP contract
    ///
    /// [UpToDateCheck](https://partner.steamgames.com/doc/webapi/ISteamApps#UpToDateCheck): `GET ISteamApps/UpToDateCheck/v1/`.
    ///
    /// # Errors
    ///
    /// Returns an error for invalid arguments or credentials, transport/HTTP failures,
    /// Steam business errors, or invalid/oversized responses.
    /// See the [shared error contract](crate#error-handling).
    ///
    /// # Examples
    ///
    /// See also the [standalone interface example](https://github.com/houseme/steam-api-web/blob/main/examples/app_builds.rs).
    ///
    /// ```no_run
    /// use steam_api_web::SteamClient;
    ///
    /// async fn example(client: &SteamClient, appid: u32, version: u32) -> steam_api_web::Result<()> {
    ///     let response = client.apps().up_to_date_check(appid, version).await?;
    ///     // Inspect the returned business fields before updating application state.
    ///     # let _ = response;
    ///     Ok(())
    /// }
    /// ```
    pub async fn up_to_date_check(&self, appid: u32, version: u32) -> Result<OpaqueResponse> {
        nonzero(appid.into(), "appid must not be zero")?;
        self.0
            .call_opaque(
                Endpoint::get("ISteamApps/UpToDateCheck/v1/", Access::Anonymous),
                Params::new().with("appid", appid).with("version", version),
            )
            .await
    }

    async fn app_read(
        &self,
        path: &'static str,
        appid: u32,
        mut params: Params,
    ) -> Result<OpaqueResponse> {
        nonzero(appid.into(), "appid must not be zero")?;
        params.push("appid", appid);
        self.0
            .call_opaque(Endpoint::get(path, Access::Publisher), params)
            .await
    }
}

fn optional_count(params: &mut Params, name: &'static str, value: Option<u32>) -> Result<()> {
    if let Some(value) = value {
        nonzero(value.into(), "count and limit must not be zero")?;
        params.push(name, value);
    }
    Ok(())
}
