// Copyright 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

//! PG-specific configuration options.

use chrono::Duration;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_with::serde_as;

use crate::ConfigurationSection;

fn default_short_lived_session_max_lifetime() -> Duration {
    Duration::hours(10)
}

fn default_long_lived_session_max_lifetime() -> Duration {
    Duration::days(30)
}

fn default_pg_session_inactivity_ttl() -> Duration {
    Duration::minutes(30)
}

/// Configuration options for the PG session expiration feature
///
/// This feature automatically expires OAuth2 sessions based on their scope:
/// - Sessions with `urn:pg:session:short-lived` scope expire after
///   `short_lived_max_lifetime` (default: 10 hours)
/// - Sessions with `urn:pg:session:long-lived` scope expire after
///   `long_lived_max_lifetime` (default: 30 days)
/// - Sessions without either scope are not affected by this feature
///
/// Additionally, sessions must have been inactive for at least `inactivity_ttl`
/// (default: 30 minutes) before being expired.
#[serde_as]
#[derive(Clone, Debug, Deserialize, JsonSchema, Serialize)]
pub struct PgSessionExpirationConfig {
    /// Whether the PG session expiration feature is enabled.
    /// Defaults to false when this section is present.
    #[serde(default)]
    pub enabled: bool,

    /// Maximum lifetime for sessions with the `urn:pg:session:short-lived` scope.
    /// Sessions older than this will be expired if they have been inactive for
    /// at least `inactivity_ttl`.
    /// Defaults to 10 hours.
    #[schemars(with = "u64", range(min = 60, max = 7_776_000))]
    #[serde(default = "default_short_lived_session_max_lifetime")]
    #[serde_as(as = "serde_with::DurationSeconds<i64>")]
    pub short_lived_max_lifetime: Duration,

    /// Maximum lifetime for sessions with the `urn:pg:session:long-lived` scope.
    /// Sessions older than this will be expired if they have been inactive for
    /// at least `inactivity_ttl`.
    /// Defaults to 30 days.
    #[schemars(with = "u64", range(min = 60, max = 7_776_000))]
    #[serde(default = "default_long_lived_session_max_lifetime")]
    #[serde_as(as = "serde_with::DurationSeconds<i64>")]
    pub long_lived_max_lifetime: Duration,

    /// Minimum inactivity time before a session can be expired.
    /// Sessions must be inactive for at least this duration before they are
    /// eligible for expiration based on their lifetime scope.
    /// Defaults to 30 minutes.
    #[schemars(with = "u64", range(min = 60, max = 86400))]
    #[serde(default = "default_pg_session_inactivity_ttl")]
    #[serde_as(as = "serde_with::DurationSeconds<i64>")]
    pub inactivity_ttl: Duration,
}

/// PG-specific configuration options
#[derive(Clone, Debug, Default, Deserialize, JsonSchema, Serialize)]
pub struct PgConfig {
    /// Configuration for automatically expiring OAuth2 sessions based on their
    /// scope (`urn:pg:session:short-lived` or `urn:pg:session:long-lived`).
    ///
    /// Disabled by default (when this section is not present).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_expiration: Option<PgSessionExpirationConfig>,
}

impl PgConfig {
    /// Returns true if the config is using all default values
    pub(crate) fn is_default(&self) -> bool {
        self.session_expiration.is_none()
    }
}

impl ConfigurationSection for PgConfig {
    const PATH: Option<&'static str> = Some("pg");
}
