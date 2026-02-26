# Changes in PG matrix-authentication-service 1.11.0+pg2 (24-02-2026)

Improvements 🙌:

- Disable GraphQL introspection

# Changes in PG matrix-authentication-service 1.11.0+pg1 (23-02-2026)

Upstream merge ✨:

- Base update to v1.11.0 (https://github.com/element-hq/matrix-authentication-service/tree/v1.11.0)

# Changes in PG matrix-authentication-service 0.0.1 (xx-xx-2025)

Upstream merge ✨:

- Base update to v1.8.0 (https://github.com/element-hq/matrix-authentication-service/tree/v1.8.0)

Features ✨:
- Auto close browser (by using deeplink) on device code flow consent, config "pg_app_scheme" under provider should be used to make it work.
- When logging out of oauth2 session that has the "urn:pg:mas-logout" scope. Linked sessions will also be terminated
- When loggin in a session expiration can be set. After x amount of time, an inactive client will be logged out
    - urn:pg:session:short-lived -> session will revoked after 10 hours (default)
    - urn:pg:session:long-lived -> session will revoked after 30 days (default)
    - session has to be inactive for at least 30 minutes (default)

Improvements 🙌:

- Use of our own PG design -> using our forked repo's & update css files.
- Repurpose "consent" & "do_register" screens by changing the content, functionality remains the same.
- Harden GraphQL endpoint against CSRF: browser POSTs now require `X-CSRF-Token`.
- Always require PKCE

Bugfix 🐛:

- GraphQL GET requests reject mutations so crafted links can no longer work.
