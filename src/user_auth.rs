// Copyright 2026 steam-api-web contributors.
// SPDX-License-Identifier: Apache-2.0
// Licensed under the Apache License, Version 2.0.
// See LICENSE or https://www.apache.org/licenses/LICENSE-2.0 for details.

//! ISteamUserAuth: ticket verification and the legacy encrypted login request.

use crate::{
    Error, Result, SteamClient, SteamId,
    request::{Access, Endpoint, Params},
    types::{nonzero, text},
};
use serde::Deserialize;
use serde_json::Value;
use std::fmt;
use zeroize::Zeroizing;

/// Access to ISteamUserAuth methods through a shared client.
#[derive(Debug, Clone, Copy)]
pub struct UserAuth<'a>(pub(crate) &'a SteamClient);

/// A Web API ticket encoded as hex, with redacted Debug and zeroized owned text.
/// Use `from_callback_bytes` with `steamworks::TicketForWebApiResponse` after its
/// result succeeds, rather than the native `steamworks::AuthTicket` handle.
pub struct WebApiTicket(Zeroizing<String>);
impl fmt::Debug for WebApiTicket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WebApiTicket([REDACTED])")
    }
}
impl WebApiTicket {
    /// Copy only the effective bytes from a steamworks-rs Web API callback.
    /// Its `ticket` vector can contain the whole SDK buffer; `ticket_len` is the
    /// valid prefix length. Reject negative, zero and out-of-bounds lengths.
    pub fn from_callback_bytes(buffer: &[u8], ticket_len: i32) -> Result<Self> {
        let len = usize::try_from(ticket_len)
            .map_err(|_| Error::InvalidParameter("invalid callback ticket length"))?;
        let bytes = buffer
            .get(..len)
            .ok_or(Error::InvalidParameter("invalid callback ticket length"))?;
        Self::from_bytes(bytes)
    }
    /// Encode raw GetAuthTicketForWebApi callback bytes as lowercase hex.
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.is_empty() {
            return Err(Error::InvalidParameter("ticket must not be empty"));
        }
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut value = Zeroizing::new(String::with_capacity(bytes.len().saturating_mul(2)));
        for &byte in bytes {
            value.push(char::from(HEX[(byte >> 4) as usize]));
            value.push(char::from(HEX[(byte & 15) as usize]));
        }
        Ok(Self(value))
    }
    /// Validate an already encoded ticket without decoding or changing its bytes.
    pub fn from_hex(value: impl Into<String>) -> Result<Self> {
        let value = Zeroizing::new(value.into());
        if value.is_empty() || value.len() % 2 != 0 || !value.bytes().all(|b| b.is_ascii_hexdigit())
        {
            return Err(Error::InvalidParameter(
                "ticket must be nonempty even-length hexadecimal",
            ));
        }
        Ok(Self(value))
    }
    /// Explicitly expose the secret hex for transmission; never log it.
    pub fn expose_hex(&self) -> &str {
        &self.0
    }
}

/// Successfully verified identity. Ban flags are optional rather than defaulting
/// absent fields to false. Applications must apply their own ban / access policy.
#[derive(Debug, Deserialize)]
pub struct AuthenticatedUser {
    /// Verified user's ID. Never taken from a client-supplied claim.
    pub steamid: SteamId,
    /// License owner's ID, when returned by Steam (e.g. Family Sharing).
    pub ownersteamid: Option<SteamId>,
    /// VAC ban flag, when returned by Steam.
    pub vacbanned: Option<bool>,
    /// Publisher ban flag, when returned by Steam.
    pub publisherbanned: Option<bool>,
}

/// Opaque response for methods whose result schema is not specified in the
/// linked official reference. Debug is redacted because login results may
/// contain credentials. Access requires an explicit call to `as_json`/`into_json`.
#[derive(Deserialize)]
#[serde(transparent)]
pub struct OpaqueResponse(Value);
impl fmt::Debug for OpaqueResponse {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("OpaqueResponse([REDACTED])")
    }
}
impl OpaqueResponse {
    pub(crate) fn from_json(value: Value) -> Self {
        Self(value)
    }

    /// Inspect Steam's original result explicitly. It may contain secrets.
    pub fn as_json(&self) -> &Value {
        &self.0
    }
    /// Take Steam's result. The caller assumes responsibility for its contents.
    pub fn into_json(self) -> Value {
        self.0
    }
}

/// Prepared legacy login material. This crate transports the encrypted bytes.
/// The reference does not specify the Steam RSA key, RSA padding, AES mode or IV;
/// obtain these values from an established compatible Steam login implementation.
/// `sessionkey` is RSA ciphertext of a random 32-byte key, not that plaintext key.
/// `encrypted_loginkey` is the hashed login key encrypted with the session key.
/// This is distinct from SteamEncryptedAppTicket and GetAuthTicketForWebApi.
pub struct AuthenticateUserRequest<'a> {
    /// Unencrypted Steam ID.
    pub steamid: SteamId,
    /// RSA-encrypted session key, sent byte-for-byte as rawbinary form data.
    pub sessionkey: &'a [u8],
    /// AES-encrypted hashed login key, sent byte-for-byte as rawbinary form data.
    pub encrypted_loginkey: &'a [u8],
}
impl fmt::Debug for AuthenticateUserRequest<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AuthenticateUserRequest")
            .field("steamid", &self.steamid)
            .finish_non_exhaustive()
    }
}

impl UserAuth<'_> {
    /// Verify a GetAuthTicketForWebApi ticket bound to a nonempty service identity.
    /// User keys work on the public host; Partner always needs a publisher key.
    /// No automatic retry is performed, even though the wire method is GET.
    pub async fn authenticate_user_ticket(
        &self,
        appid: u32,
        ticket: &WebApiTicket,
        identity: &str,
    ) -> Result<AuthenticatedUser> {
        const PATH: &str = "ISteamUserAuth/AuthenticateUserTicket/v1/";
        nonzero(appid.into(), "appid must not be zero")?;
        text(identity, "identity must not be empty or contain NUL")?;
        #[derive(Deserialize)]
        struct Verified {
            result: String,
            #[serde(flatten)]
            user: AuthenticatedUser,
        }
        let response: Verified = self
            .0
            .call(
                Endpoint::get_once(PATH, Access::User),
                Params::new()
                    .with("appid", appid)
                    .with("ticket", ticket.expose_hex())
                    .with("identity", identity),
                &["response", "params"],
            )
            .await?;
        if response.result != "OK" || response.user.steamid.0 == 0 {
            return Err(Error::Api {
                endpoint: PATH,
                code: None,
            });
        }
        Ok(response.user)
    }
    /// Send prepared encrypted legacy credentials, with no automatic retry.
    /// Raw bytes use percent encoding, never a lossy UTF-8 or hex conversion.
    /// For new integrations prefer `authenticate_user_ticket`.
    pub async fn authenticate_user(
        &self,
        request: &AuthenticateUserRequest<'_>,
    ) -> Result<OpaqueResponse> {
        nonzero(request.steamid.0, "steamid must not be zero")?;
        if request.sessionkey.is_empty() || request.encrypted_loginkey.is_empty() {
            return Err(Error::InvalidParameter(
                "encrypted login material must not be empty",
            ));
        }
        let mut params = Params::new().with("steamid", request.steamid);
        params.push_bytes("sessionkey", request.sessionkey);
        params.push_bytes("encrypted_loginkey", request.encrypted_loginkey);
        self.0
            .call_opaque(
                Endpoint::post("ISteamUserAuth/AuthenticateUser/v1/", Access::Anonymous),
                params,
            )
            .await
    }
}
