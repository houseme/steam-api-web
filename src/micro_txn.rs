// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! Steam Wallet purchases and recurring billing, with explicit environment choice.
//!
//! Select [`SteamClient::micro_txn_sandbox`] while developing. Production is only
//! selected by [`SteamClient::micro_txn_production`]; requests never switch or
//! fall back between the two interfaces. Both require a trusted server and a
//! publisher key with Microtransaction permissions, which Steam checks remotely.
//!
//! Money uses signed wire integers in Steam hundredths of the supplied currency,
//! including JPY and KWD. Steam validates each currency's required increments.
//! A line amount is the complete line total, not the unit price. No conversion,
//! multiplication, rounding, automatic finalization, or entitlement grant occurs.
//! Every mutation is sent once, including requests carrying a unique order ID.
//! On an ambiguous FinalizeTxn result, reconcile with QueryTxn or GetReport.
//! For an ambiguous InitTxn result, Valve instructs abandoning that transaction
//! and creating another. A successful InitTxn creates an order awaiting approval;
//! ProcessAgreement initiates billing rather than confirming completed payment.

use crate::{
    Error, Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::{nonzero, text},
    user_auth::OpaqueResponse,
};
use std::net::Ipv4Addr;

/// Which fixed Steam billing interface receives a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicroTxnEnvironment {
    /// Dummy billing database; no actual wallet funds are withdrawn.
    Sandbox,
    /// Production billing, which can capture or refund real funds.
    Production,
}

/// One explicit billing environment implementing all ten documented methods.
#[derive(Debug, Clone, Copy)]
pub struct MicroTxn<'a> {
    client: &'a SteamClient,
    environment: MicroTxnEnvironment,
}

/// A recurring agreement owned by a user within one application.
#[derive(Debug, Clone, Copy)]
pub struct AgreementRequest {
    /// Application ID.
    pub appid: u32,
    /// User whose agreement is addressed.
    pub steamid: SteamId,
    /// Steam's full 64-bit agreement ID.
    pub agreementid: u64,
}

/// Extend a Steam-managed subscription's next processing date.
#[derive(Debug, Clone, Copy)]
pub struct AdjustAgreementRequest<'a> {
    /// Agreement being extended.
    pub agreement: AgreementRequest,
    /// New next processing date in `YYYYMMDD` format.
    pub nextprocessdate: &'a str,
    /// Optional optimistic concurrency guard in `YYYYMMDD` format.
    pub oldnextprocessdate: Option<&'a str>,
}

/// An existing order used for finalization or a full-order refund.
#[derive(Debug, Clone, Copy)]
pub struct OrderRequest {
    /// Application ID.
    pub appid: u32,
    /// Merchant's unique 64-bit order ID, preserved without rounding.
    pub orderid: u64,
}

/// A user and application for querying billing agreements.
#[derive(Debug, Clone, Copy)]
pub struct MicroTxnUserRequest {
    /// Application ID.
    pub appid: u32,
    /// User's full 64-bit Steam ID.
    pub steamid: SteamId,
}

/// User purchasing metadata; web purchases should include the public IPv4 address.
#[derive(Debug, Clone, Copy)]
pub struct PurchasingUserRequest<'a> {
    /// User and application.
    pub user: MicroTxnUserRequest,
    /// Optional IPv4 hint, required by Steam for users without a client session.
    pub ipaddress: Option<&'a str>,
}

/// Identifiers for explicit order reconciliation. At least one ID is required.
#[derive(Debug, Clone, Copy)]
pub struct QueryTxnRequest {
    /// Application ID.
    pub appid: u32,
    /// Merchant order ID, if known.
    pub orderid: Option<u64>,
    /// Steam transaction ID, if known. Use this for Steam Store renewals with order ID zero.
    pub transid: Option<u64>,
}

/// Documented transaction report categories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicroTxnReportType {
    /// Sales made through the game's purchasing flow.
    GameSales,
    /// Steam Store transactions associated with in-game items.
    SteamStoreSales,
    /// Settlement updates.
    Settlement,
    /// Chargeback updates.
    Chargeback,
    /// Recurring subscription updates.
    Subscription,
}

impl MicroTxnReportType {
    fn wire(self) -> &'static str {
        match self {
            Self::GameSales => "GAMESALES",
            Self::SteamStoreSales => "STEAMSTORESALES",
            Self::Settlement => "SETTLEMENT",
            Self::Chargeback => "CHARGEBACK",
            Self::Subscription => "SUBSCRIPTION",
        }
    }
}

/// A single reconciliation batch. This API is for entitlement reconciliation;
/// Valve directs financial reporting to IPartnerFinancialsService instead.
#[derive(Debug, Clone, Copy)]
pub struct MicroTxnReportRequest<'a> {
    /// Application ID.
    pub appid: u32,
    /// RFC 3339 UTC update-time cursor. Start with a known checkpoint; advance
    /// using the latest returned transaction update time. Keep it when empty.
    pub time: &'a str,
    /// Optional category; omission leaves Steam's default unchanged.
    pub report_type: Option<MicroTxnReportType>,
    /// Optional result limit in 1..=50,000; omitted defaults to 1000.
    /// A short page does not establish exhaustion, so no auto-pagination occurs.
    pub maxresults: Option<u32>,
}

/// How the user authorizes a newly initiated purchase.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MicroTxnUserSession {
    /// The user authorizes through the logged-in Steam client's overlay.
    Client,
    /// The user authorizes through the returned web URL; an IPv4 address is required.
    Web,
}

/// Who initiates charges for a recurring agreement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecurringBillingType {
    /// Steam automatically bills the user.
    Steam,
    /// The game calls ProcessAgreement for each charge.
    Game,
}

/// A recurring agreement's billing period.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecurringPeriod {
    /// Daily period.
    Day,
    /// Weekly period.
    Week,
    /// Monthly period.
    Month,
    /// Yearly period.
    Year,
}

/// The four recurring fields are grouped so a partial agreement cannot be sent.
#[derive(Debug, Clone, Copy)]
pub struct RecurringBilling {
    /// Steam-managed or game-managed recurring charges.
    pub billingtype: RecurringBillingType,
    /// Period for the agreement.
    pub period: RecurringPeriod,
    /// Documented frequency in 1..=255.
    pub frequency: u8,
    /// Complete amount of a future charge in Steam hundredths of the currency.
    pub recurringamt: i64,
}

/// One cart line for InitTxn v3. Its signed integer amounts are sent unchanged.
#[derive(Debug, Clone, Copy)]
pub struct MicroTxnItem<'a> {
    /// Merchant-defined 32-bit item ID; zero is not reserved by this wrapper.
    pub itemid: u32,
    /// Positive quantity, using the documented signed 16-bit wire range.
    pub qty: i16,
    /// Total line cost in Steam hundredths of the currency, including every quantity.
    pub amount: i64,
    /// User-visible description, at most 128 Unicode characters.
    pub description: &'a str,
    /// Optional reporting category, at most 64 Unicode characters.
    pub category: Option<&'a str>,
    /// Optional merchant-defined bundle ID associated with this item.
    pub associated_bundle: Option<u32>,
    /// Optional complete recurring-billing terms.
    pub recurring: Option<RecurringBilling>,
}

/// Optional bundle metadata sharing the same ID space as merchant item IDs.
#[derive(Debug, Clone, Copy)]
pub struct MicroTxnBundle<'a> {
    /// Merchant-defined bundle ID.
    pub bundleid: u32,
    /// Number of bundles in the cart.
    pub bundle_qty: u32,
    /// User-visible bundle description, at most 128 Unicode characters.
    pub bundle_desc: &'a str,
    /// Optional reporting category, at most 64 Unicode characters.
    pub bundle_category: Option<&'a str>,
}

/// Borrowed cart for creating a transaction awaiting the user's authorization.
#[derive(Debug, Clone, Copy)]
pub struct InitTxnRequest<'a> {
    /// Application ID.
    pub appid: u32,
    /// Unique merchant order ID; persist it before calling Steam.
    pub orderid: u64,
    /// User making the purchase.
    pub steamid: SteamId,
    /// Item-description language code; Steam validates its supported languages.
    pub language: &'a str,
    /// Three-letter uppercase ISO 4217 currency code.
    pub currency: &'a str,
    /// At least one cart line. Its length becomes itemcount.
    pub items: &'a [MicroTxnItem<'a>],
    /// Optional bundle metadata. An empty slice omits bundlecount.
    pub bundles: &'a [MicroTxnBundle<'a>],
    /// Omit to retain Steam's default client session.
    pub usersession: Option<MicroTxnUserSession>,
    /// Public IPv4 address; required for an explicit web session.
    pub ipaddress: Option<&'a str>,
}

/// Start a game-managed recurring billing cycle; success does not confirm payment.
#[derive(Debug, Clone, Copy)]
pub struct ProcessAgreementRequest<'a> {
    /// Agreement whose billing cycle will be initiated.
    pub agreement: AgreementRequest,
    /// Merchant order ID; zero is explicitly valid for Steam Store subscriptions.
    pub orderid: u64,
    /// Immediate charge in Steam hundredths, with the signed 32-bit wire range.
    pub amount: i32,
    /// Three-letter uppercase ISO 4217 currency code.
    pub currency: &'a str,
}

impl<'a> MicroTxn<'a> {
    pub(crate) fn sandbox(client: &'a SteamClient) -> Self {
        Self {
            client,
            environment: MicroTxnEnvironment::Sandbox,
        }
    }

    pub(crate) fn production(client: &'a SteamClient) -> Self {
        Self {
            client,
            environment: MicroTxnEnvironment::Production,
        }
    }

    /// Inspect the explicitly selected environment without sending a request.
    pub fn environment(&self) -> MicroTxnEnvironment {
        self.environment
    }

    /// Extend a Steam-managed agreement using optional optimistic concurrency.
    pub async fn adjust_agreement(
        &self,
        request: AdjustAgreementRequest<'_>,
    ) -> Result<OpaqueResponse> {
        validate_agreement(request.agreement)?;
        if !valid_date(request.nextprocessdate)
            || request
                .oldnextprocessdate
                .is_some_and(|value| !valid_date(value))
        {
            return Err(Error::InvalidParameter(
                "agreement dates must use valid YYYYMMDD values",
            ));
        }
        if request
            .oldnextprocessdate
            .is_some_and(|old| request.nextprocessdate < old)
        {
            return Err(Error::InvalidParameter(
                "nextprocessdate must not precede oldnextprocessdate",
            ));
        }
        let mut params =
            agreement_params(request.agreement).with("nextprocessdate", request.nextprocessdate);
        if let Some(old) = request.oldnextprocessdate {
            params.push("oldnextprocessdate", old);
        }
        self.send(MicroMethod::AdjustAgreement, params).await
    }

    /// Cancel an agreement, without replaying a timeout or transient failure.
    pub async fn cancel_agreement(&self, request: AgreementRequest) -> Result<OpaqueResponse> {
        validate_agreement(request)?;
        self.send(MicroMethod::CancelAgreement, agreement_params(request))
            .await
    }

    /// Capture an already authorized order, sent once. Reconcile ambiguous results
    /// through QueryTxn or GetReport before deciding entitlement changes.
    pub async fn finalize_txn(&self, request: OrderRequest) -> Result<OpaqueResponse> {
        validate_order(request)?;
        self.send(MicroMethod::FinalizeTxn, order_params(request))
            .await
    }

    /// Read one v5 reconciliation batch. Preserve unknown statuses, full IDs,
    /// individual item outcomes, and timestamps in the original redacted JSON.
    pub async fn get_report(&self, request: MicroTxnReportRequest<'_>) -> Result<OpaqueResponse> {
        nonzero(request.appid.into(), "appid must not be zero")?;
        if !valid_utc_time(request.time) {
            return Err(Error::InvalidParameter(
                "time must be a valid RFC 3339 UTC timestamp",
            ));
        }
        if request
            .maxresults
            .is_some_and(|count| !(1..=50_000).contains(&count))
        {
            return Err(Error::InvalidParameter("maxresults must be in 1..=50000"));
        }
        let mut params = Params::new()
            .with("appid", request.appid)
            .with("time", request.time);
        if let Some(report_type) = request.report_type {
            params.push("type", report_type.wire());
        }
        if let Some(maxresults) = request.maxresults {
            params.push("maxresults", maxresults);
        }
        self.send(MicroMethod::GetReport, params).await
    }

    /// Read current agreement status. Use GetReport for successful renewal charges;
    /// a canceled agreement may still have previously paid entitlement remaining.
    pub async fn get_user_agreement_info(
        &self,
        request: MicroTxnUserRequest,
    ) -> Result<OpaqueResponse> {
        validate_user(request)?;
        self.send(MicroMethod::GetUserAgreementInfo, user_params(request))
            .await
    }

    /// Read wallet currency, country, state, and purchasing status (v2).
    pub async fn get_user_info(
        &self,
        request: PurchasingUserRequest<'_>,
    ) -> Result<OpaqueResponse> {
        validate_user(request.user)?;
        let mut params = user_params(request.user);
        if let Some(ipaddress) = request.ipaddress {
            validate_ip(ipaddress)?;
            params.push("ipaddress", ipaddress);
        }
        self.send(MicroMethod::GetUserInfo, params).await
    }

    /// Create a v3 cart awaiting approval. This method does not finalize the order
    /// or follow the returned authorization URL. All writes are single-send.
    pub async fn init_txn(&self, request: InitTxnRequest<'_>) -> Result<OpaqueResponse> {
        validate_order(OrderRequest {
            appid: request.appid,
            orderid: request.orderid,
        })?;
        nonzero(request.steamid.raw(), "steamid must not be zero")?;
        text(
            request.language,
            "language must not be empty or contain NUL",
        )?;
        validate_currency(request.currency)?;
        let count = cart_count(request.items.len(), false)?;
        let mut params = Params::new()
            .with("appid", request.appid)
            .with("orderid", request.orderid)
            .with("steamid", request.steamid)
            .with("language", request.language)
            .with("currency", request.currency)
            .with("itemcount", count);
        if let Some(session) = request.usersession {
            params.push(
                "usersession",
                match session {
                    MicroTxnUserSession::Client => "client",
                    MicroTxnUserSession::Web => "web",
                },
            );
        }
        if request.usersession == Some(MicroTxnUserSession::Web) && request.ipaddress.is_none() {
            return Err(Error::InvalidParameter(
                "web usersession requires ipaddress",
            ));
        }
        if let Some(ipaddress) = request.ipaddress {
            validate_ip(ipaddress)?;
            params.push("ipaddress", ipaddress);
        }
        for (index, item) in request.items.iter().enumerate() {
            encode_item(&mut params, index, item)?;
        }
        if !request.bundles.is_empty() {
            params.push("bundlecount", cart_count(request.bundles.len(), true)?);
            for (index, bundle) in request.bundles.iter().enumerate() {
                nonzero(bundle.bundle_qty.into(), "bundle_qty must not be zero")?;
                bounded_text(bundle.bundle_desc, 128, true)?;
                if let Some(category) = bundle.bundle_category {
                    bounded_text(category, 64, false)?;
                }
                params.push_indexed("bundleid", index, bundle.bundleid);
                params.push_indexed("bundle_qty", index, bundle.bundle_qty);
                params.push_indexed("bundle_desc", index, bundle.bundle_desc);
                if let Some(category) = bundle.bundle_category {
                    params.push_indexed("bundle_category", index, category);
                }
            }
        }
        self.send(MicroMethod::InitTxn, params).await
    }

    /// Initiate a game-managed billing cycle once. An accepted request can still
    /// fail to charge; reconcile the actual cycle through GetReport.
    pub async fn process_agreement(
        &self,
        request: ProcessAgreementRequest<'_>,
    ) -> Result<OpaqueResponse> {
        validate_agreement(request.agreement)?;
        validate_currency(request.currency)?;
        self.send(
            MicroMethod::ProcessAgreement,
            agreement_params(request.agreement)
                .with("orderid", request.orderid)
                .with("amount", request.amount)
                .with("currency", request.currency),
        )
        .await
    }

    /// Query an order or Steam transaction without narrowing its identifiers.
    pub async fn query_txn(&self, request: QueryTxnRequest) -> Result<OpaqueResponse> {
        nonzero(request.appid.into(), "appid must not be zero")?;
        if request.orderid.is_none() && request.transid.is_none() {
            return Err(Error::InvalidParameter("query requires orderid or transid"));
        }
        if let Some(transid) = request.transid {
            nonzero(transid, "transid must not be zero")?;
        }
        if request.orderid == Some(0) && request.transid.is_none() {
            return Err(Error::InvalidParameter(
                "zero orderid requires a Steam transaction ID",
            ));
        }
        let mut params = Params::new().with("appid", request.appid);
        if let Some(orderid) = request.orderid {
            params.push("orderid", orderid);
        }
        if let Some(transid) = request.transid {
            params.push("transid", transid);
        }
        self.send(MicroMethod::QueryTxn, params).await
    }

    /// Request a full refund for an order, once. Partial-order refunds are not
    /// supported by this API. Inspect reconciliation reports for later reversals.
    pub async fn refund_txn(&self, request: OrderRequest) -> Result<OpaqueResponse> {
        validate_order(request)?;
        self.send(MicroMethod::RefundTxn, order_params(request))
            .await
    }

    async fn send(&self, method: MicroMethod, params: Params) -> Result<OpaqueResponse> {
        let path = method.path(self.environment);
        let endpoint = if method.is_read() {
            Endpoint::get(path, Access::Publisher)
        } else {
            Endpoint::post(path, Access::Publisher)
        };
        self.client
            .call_opaque(endpoint.micro_transaction(), params)
            .await
    }
}

#[derive(Clone, Copy)]
enum MicroMethod {
    AdjustAgreement,
    CancelAgreement,
    FinalizeTxn,
    GetReport,
    GetUserAgreementInfo,
    GetUserInfo,
    InitTxn,
    ProcessAgreement,
    QueryTxn,
    RefundTxn,
}

impl MicroMethod {
    fn is_read(self) -> bool {
        matches!(
            self,
            Self::GetReport | Self::GetUserAgreementInfo | Self::GetUserInfo | Self::QueryTxn
        )
    }

    fn path(self, environment: MicroTxnEnvironment) -> &'static str {
        match (environment, self) {
            (MicroTxnEnvironment::Sandbox, Self::AdjustAgreement) => {
                "ISteamMicroTxnSandbox/AdjustAgreement/v1/"
            }
            (MicroTxnEnvironment::Sandbox, Self::CancelAgreement) => {
                "ISteamMicroTxnSandbox/CancelAgreement/v1/"
            }
            (MicroTxnEnvironment::Sandbox, Self::FinalizeTxn) => {
                "ISteamMicroTxnSandbox/FinalizeTxn/v2/"
            }
            (MicroTxnEnvironment::Sandbox, Self::GetReport) => {
                "ISteamMicroTxnSandbox/GetReport/v5/"
            }
            (MicroTxnEnvironment::Sandbox, Self::GetUserAgreementInfo) => {
                "ISteamMicroTxnSandbox/GetUserAgreementInfo/v2/"
            }
            (MicroTxnEnvironment::Sandbox, Self::GetUserInfo) => {
                "ISteamMicroTxnSandbox/GetUserInfo/v2/"
            }
            (MicroTxnEnvironment::Sandbox, Self::InitTxn) => "ISteamMicroTxnSandbox/InitTxn/v3/",
            (MicroTxnEnvironment::Sandbox, Self::ProcessAgreement) => {
                "ISteamMicroTxnSandbox/ProcessAgreement/v1/"
            }
            (MicroTxnEnvironment::Sandbox, Self::QueryTxn) => "ISteamMicroTxnSandbox/QueryTxn/v3/",
            (MicroTxnEnvironment::Sandbox, Self::RefundTxn) => {
                "ISteamMicroTxnSandbox/RefundTxn/v2/"
            }
            (MicroTxnEnvironment::Production, Self::AdjustAgreement) => {
                "ISteamMicroTxn/AdjustAgreement/v1/"
            }
            (MicroTxnEnvironment::Production, Self::CancelAgreement) => {
                "ISteamMicroTxn/CancelAgreement/v1/"
            }
            (MicroTxnEnvironment::Production, Self::FinalizeTxn) => {
                "ISteamMicroTxn/FinalizeTxn/v2/"
            }
            (MicroTxnEnvironment::Production, Self::GetReport) => "ISteamMicroTxn/GetReport/v5/",
            (MicroTxnEnvironment::Production, Self::GetUserAgreementInfo) => {
                "ISteamMicroTxn/GetUserAgreementInfo/v2/"
            }
            (MicroTxnEnvironment::Production, Self::GetUserInfo) => {
                "ISteamMicroTxn/GetUserInfo/v2/"
            }
            (MicroTxnEnvironment::Production, Self::InitTxn) => "ISteamMicroTxn/InitTxn/v3/",
            (MicroTxnEnvironment::Production, Self::ProcessAgreement) => {
                "ISteamMicroTxn/ProcessAgreement/v1/"
            }
            (MicroTxnEnvironment::Production, Self::QueryTxn) => "ISteamMicroTxn/QueryTxn/v3/",
            (MicroTxnEnvironment::Production, Self::RefundTxn) => "ISteamMicroTxn/RefundTxn/v2/",
        }
    }
}

fn validate_user(request: MicroTxnUserRequest) -> Result<()> {
    nonzero(request.appid.into(), "appid must not be zero")?;
    nonzero(request.steamid.raw(), "steamid must not be zero")
}
fn validate_agreement(request: AgreementRequest) -> Result<()> {
    validate_user(MicroTxnUserRequest {
        appid: request.appid,
        steamid: request.steamid,
    })?;
    nonzero(request.agreementid, "agreementid must not be zero")
}
fn validate_order(request: OrderRequest) -> Result<()> {
    nonzero(request.appid.into(), "appid must not be zero")?;
    nonzero(request.orderid, "orderid must not be zero")
}
fn user_params(request: MicroTxnUserRequest) -> Params {
    Params::new()
        .with("appid", request.appid)
        .with("steamid", request.steamid)
}
fn agreement_params(request: AgreementRequest) -> Params {
    user_params(MicroTxnUserRequest {
        appid: request.appid,
        steamid: request.steamid,
    })
    .with("agreementid", request.agreementid)
}
fn order_params(request: OrderRequest) -> Params {
    Params::new()
        .with("appid", request.appid)
        .with("orderid", request.orderid)
}
fn validate_currency(value: &str) -> Result<()> {
    if value.len() != 3 || !value.bytes().all(|byte| byte.is_ascii_uppercase()) {
        return Err(Error::InvalidParameter(
            "currency must be a three-letter uppercase ISO 4217 code",
        ));
    }
    Ok(())
}
fn validate_ip(value: &str) -> Result<()> {
    if value.parse::<Ipv4Addr>().is_err() {
        return Err(Error::InvalidParameter("ipaddress must be an IPv4 address"));
    }
    Ok(())
}
fn cart_count(count: usize, allow_empty: bool) -> Result<u32> {
    let count =
        u32::try_from(count).map_err(|_| Error::InvalidParameter("cart count must fit uint32"))?;
    if count == 0 && !allow_empty {
        return Err(Error::InvalidParameter("items must not be empty"));
    }
    Ok(count)
}
fn bounded_text(value: &str, max: usize, required: bool) -> Result<()> {
    if value.contains('\0') || value.chars().count() > max || (required && value.trim().is_empty())
    {
        return Err(Error::InvalidParameter(
            "description or category exceeds its character bound or contains invalid text",
        ));
    }
    Ok(())
}
fn encode_item(params: &mut Params, index: usize, item: &MicroTxnItem<'_>) -> Result<()> {
    if item.qty <= 0 {
        return Err(Error::InvalidParameter(
            "qty must be positive within the signed 16-bit range",
        ));
    }
    bounded_text(item.description, 128, true)?;
    if let Some(category) = item.category {
        bounded_text(category, 64, false)?;
    }
    params.push_indexed("itemid", index, item.itemid);
    params.push_indexed("qty", index, item.qty);
    params.push_indexed("amount", index, item.amount);
    params.push_indexed("description", index, item.description);
    if let Some(category) = item.category {
        params.push_indexed("category", index, category);
    }
    if let Some(bundle) = item.associated_bundle {
        params.push_indexed("associated_bundle", index, bundle);
    }
    if let Some(recurring) = item.recurring {
        if recurring.frequency == 0 {
            return Err(Error::InvalidParameter(
                "recurring frequency must be in 1..=255",
            ));
        }
        params.push_indexed(
            "billingtype",
            index,
            match recurring.billingtype {
                RecurringBillingType::Steam => "Steam",
                RecurringBillingType::Game => "Game",
            },
        );
        params.push_indexed(
            "period",
            index,
            match recurring.period {
                RecurringPeriod::Day => "Day",
                RecurringPeriod::Week => "Week",
                RecurringPeriod::Month => "Month",
                RecurringPeriod::Year => "Year",
            },
        );
        params.push_indexed("frequency", index, recurring.frequency);
        params.push_indexed("recurringamt", index, recurring.recurringamt);
    }
    Ok(())
}
fn valid_date(value: &str) -> bool {
    let bytes = value.as_bytes();
    if bytes.len() != 8 || !bytes.iter().all(u8::is_ascii_digit) {
        return false;
    }
    let number = |start: usize, end: usize| {
        bytes[start..end]
            .iter()
            .fold(0u32, |n, byte| n * 10 + u32::from(byte - b'0'))
    };
    let year = number(0, 4);
    let month = number(4, 6);
    let day = number(6, 8);
    let leap = year.is_multiple_of(4) && (!year.is_multiple_of(100) || year.is_multiple_of(400));
    let days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return false,
    };
    year != 0 && (1..=days).contains(&day)
}
fn valid_utc_time(value: &str) -> bool {
    let Some(value) = value
        .strip_suffix('Z')
        .or_else(|| value.strip_suffix('z'))
        .or_else(|| value.strip_suffix("+00:00"))
    else {
        return false;
    };
    let bytes = value.as_bytes();
    if bytes.len() < 19
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !matches!(bytes[10], b'T' | b't')
        || bytes[13] != b':'
        || bytes[16] != b':'
    {
        return false;
    }
    if bytes[..19]
        .iter()
        .enumerate()
        .any(|(index, byte)| ![4, 7, 10, 13, 16].contains(&index) && !byte.is_ascii_digit())
    {
        return false;
    }
    let date = [
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[5], bytes[6], bytes[8], bytes[9],
    ];
    let Ok(date) = std::str::from_utf8(&date) else {
        return false;
    };
    if !valid_date(date) {
        return false;
    }
    let number = |start: usize| (bytes[start] - b'0') * 10 + bytes[start + 1] - b'0';
    if number(11) > 23 || number(14) > 59 || number(17) > 60 {
        return false;
    }
    bytes.len() == 19
        || (bytes[19] == b'.' && bytes.len() > 20 && bytes[20..].iter().all(u8::is_ascii_digit))
}
