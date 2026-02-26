// TODO: PG license 

//! PG-specific OAuth2 scope tokens.
//!
//! These scopes are used to control PG-specific behavior in the Matrix
//! Authentication Service.

use oauth2_types::scope::ScopeToken;

/// `urn:pg:mas-logout`
///
/// When a session with this scope revokes its access or refresh token, MAS will
/// also end the browser session (MAS session) that the OAuth2 session is linked
/// to, along with all other OAuth2 sessions linked to that browser session.
pub const PG_MAS_LOGOUT: ScopeToken = ScopeToken::from_static("urn:pg:mas-logout");

/// `urn:pg:session:short-lived`
///
/// Sessions with this scope are considered "short-lived" and will be
/// automatically expired after a configurable duration (default: 10 hours) if
/// they have been inactive for at least the configured inactivity TTL (default: 30 minutes).
pub const PG_SESSION_SHORT_LIVED: ScopeToken = ScopeToken::from_static("urn:pg:session:short-lived");

/// `urn:pg:session:long-lived`
///
/// Sessions with this scope are considered "long-lived" and will be
/// automatically expired after a configurable duration (default: 30 days) if
/// they have been inactive for at least the configured inactivity TTL (default: 30 minutes).
pub const PG_SESSION_LONG_LIVED: ScopeToken = ScopeToken::from_static("urn:pg:session:long-lived");
