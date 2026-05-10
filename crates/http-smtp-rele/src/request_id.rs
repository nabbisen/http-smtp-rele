//! Application-owned request identifier.
//!
//! Implements RFC 036/086: `request_id` is an opaque, application-owned identifier.
//! The canonical external format is `req_` followed by a ULID.
//!
//! Clients must treat `request_id` as an opaque string.
//! They must not parse the timestamp component or depend on internal format details.

use std::{fmt, str::FromStr};

use serde::{Deserialize, Serialize};
use ulid::Ulid;

/// An opaque, application-owned submission request identifier.
///
/// External format: `req_` + ULID (26 uppercase base-32 characters).
/// Example: `req_01HX7Q9V6R6W9V8Y5E3E6E7M9A`
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RequestId(String);

impl RequestId {
    /// Generate a new, unique `RequestId`.
    pub fn new() -> Self {
        RequestId(format!("req_{}", Ulid::new()))
    }

    /// Return the canonical string representation.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RequestId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for RequestId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct RequestIdParseError;

impl fmt::Display for RequestIdParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("invalid request_id: must be req_<ULID>")
    }
}

impl FromStr for RequestId {
    type Err = RequestIdParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let ulid_part = s.strip_prefix("req_").ok_or(RequestIdParseError)?;
        Ulid::from_string(ulid_part).map_err(|_| RequestIdParseError)?;
        Ok(RequestId(s.to_string()))
    }
}

// Implement AsRef<str> for use as HashMap key etc.
impl AsRef<str> for RequestId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_id_has_req_prefix() {
        let id = RequestId::new();
        assert!(id.as_str().starts_with("req_"), "got: {id}");
    }

    #[test]
    fn display_roundtrips_via_fromstr() {
        let id = RequestId::new();
        let s = id.to_string();
        let parsed: RequestId = s.parse().unwrap();
        assert_eq!(id, parsed);
    }

    #[test]
    fn plain_uuid_rejected() {
        let result = "550e8400-e29b-41d4-a716-446655440000".parse::<RequestId>();
        assert!(result.is_err());
    }

    #[test]
    fn wrong_prefix_rejected() {
        let result = "id_01HX7Q9V6R6W9V8Y5E3E6E7M9A".parse::<RequestId>();
        assert!(result.is_err());
    }

    #[test]
    fn two_ids_are_unique() {
        assert_ne!(RequestId::new(), RequestId::new());
    }
}
