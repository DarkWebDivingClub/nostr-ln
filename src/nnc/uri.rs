//! The `nostr+nodecontrol://` connection URI.
//!
//! **It carries no secret.** NWC's URI hands you a key the wallet service
//! generated, and holding it *is* the permission. NIP-XX says the client
//! uses its own keypair and the owner publishes a grant for that pubkey —
//! so the URI says who and where, and your signer says who you are.
//!
//! A consequence worth handling rather than discovering: a client can
//! connect perfectly and be refused every method, because a URI is not a
//! credential here.

use std::fmt;
use std::str::FromStr;

use nostr::key::PublicKey;

/// The scheme NIP-XX defines.
pub const SCHEME: &str = "nostr+nodecontrol";

/// A parsed connection URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeControlUri {
    /// The node service's pubkey.
    pub service: PublicKey,
    /// Where it listens. At least one.
    pub relays: Vec<String>,
}

/// Why a URI is not a `nostr+nodecontrol://` URI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UriError {
    /// Not the `nostr+nodecontrol` scheme.
    WrongScheme(String),
    /// The service pubkey is missing or malformed.
    BadServiceKey,
    /// No `relay` parameter.
    NoRelay,
    /// Carries a `secret`.
    ///
    /// That is an NWC URI. NNC clients sign with their own key, and
    /// accepting a URI-supplied secret would silently give the caller an
    /// identity the owner never granted anything to.
    CarriesSecret,
}

impl fmt::Display for UriError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WrongScheme(s) => write!(f, "expected {SCHEME}://, got {s}"),
            Self::BadServiceKey => f.write_str("the node service pubkey is missing or malformed"),
            Self::NoRelay => f.write_str("no relay parameter"),
            Self::CarriesSecret => f.write_str(
                "this URI carries a secret, so it is an NWC URI — an NNC client signs with its own key",
            ),
        }
    }
}

impl std::error::Error for UriError {}

impl FromStr for NodeControlUri {
    type Err = UriError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let rest = s
            .strip_prefix(&format!("{SCHEME}://"))
            .ok_or_else(|| UriError::WrongScheme(s.split("://").next().unwrap_or(s).to_string()))?;

        let (key, query) = rest.split_once('?').unwrap_or((rest, ""));
        let service = PublicKey::from_hex(key).map_err(|_| UriError::BadServiceKey)?;

        let mut relays = Vec::new();
        for pair in query.split('&').filter(|p| !p.is_empty()) {
            let (k, v) = pair.split_once('=').unwrap_or((pair, ""));
            match k {
                "relay" => relays.push(percent_decode(v)),
                "secret" => return Err(UriError::CarriesSecret),
                _ => {}
            }
        }
        if relays.is_empty() {
            return Err(UriError::NoRelay);
        }
        Ok(Self { service, relays })
    }
}

impl fmt::Display for NodeControlUri {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{SCHEME}://{}", self.service.to_hex())?;
        for (i, r) in self.relays.iter().enumerate() {
            write!(f, "{}relay={}", if i == 0 { '?' } else { '&' }, percent_encode(r))?;
        }
        Ok(())
    }
}

fn percent_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = String::with_capacity(s.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v as char);
                i += 3;
                continue;
            }
        }
        out.push(b[i] as char);
        i += 1;
    }
    out
}

fn percent_encode(s: &str) -> String {
    s.bytes()
        .map(|c| match c {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (c as char).to_string()
            }
            other => format!("%{other:02X}"),
        })
        .collect()
}
