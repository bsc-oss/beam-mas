// Copyright 2024, 2025 New Vector Ltd.
// Copyright 2023, 2024 The Matrix.org Foundation C.I.C.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

use std::{sync::Arc, time::Duration};

use anyhow::Context;
use axum::{
    Form,
    extract::{Path, State},
    response::{Html, IntoResponse, Response, Redirect},
};
use axum_extra::TypedHeader;
use mas_axum_utils::{
    InternalError,
    cookies::CookieJar,
    csrf::{CsrfExt, ProtectedForm},
};
use mas_data_model::{BoxClock, BoxRng, MatrixUser};
use mas_matrix::HomeserverConnection;
use mas_policy::Policy;
 // PG_CHANGED: post-logout action
use mas_router::{PostAuthAction, UrlBuilder};
use mas_storage::BoxRepository;
use mas_templates::{DeviceConsentContext, PolicyViolationContext, TemplateContext, Templates};
use serde::Deserialize;
use tracing::warn;
use ulid::Ulid;

use crate::{
    BoundActivityTracker, PreferredLanguage,
    pg_user_info::fetch_user_email,
    session::{SessionOrFallback, count_user_sessions_for_limiting, load_session_or_fallback},
};

#[derive(Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
enum Action {
    Consent,
    Reject,
}

#[derive(Deserialize, Debug)]
pub(crate) struct ConsentForm {
    action: Action,
}

#[tracing::instrument(name = "handlers.oauth2.device.consent.get", skip_all)]
pub(crate) async fn get(
    mut rng: BoxRng,
    clock: BoxClock,
    PreferredLanguage(locale): PreferredLanguage,
    State(templates): State<Templates>,
    State(url_builder): State<UrlBuilder>,
    State(homeserver): State<Arc<dyn HomeserverConnection>>,
    mut repo: BoxRepository,
    mut policy: Policy,
    activity_tracker: BoundActivityTracker,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    cookie_jar: CookieJar,
    Path(grant_id): Path<Ulid>,
) -> Result<Response, InternalError> {
     // PG_CHANGED: post-logout action
    let post_auth_action = Some(PostAuthAction::continue_device_code_grant(grant_id));
    let (cookie_jar, maybe_session) = match load_session_or_fallback(
        cookie_jar, &clock, &mut rng, &templates, &locale, &mut repo, post_auth_action,
    )
    .await?
    {
        SessionOrFallback::MaybeSession {
            cookie_jar,
            maybe_session,
            ..
        } => (cookie_jar, maybe_session),
        SessionOrFallback::Fallback { response } => return Ok(response),
    };

    let (csrf_token, cookie_jar) = cookie_jar.csrf_token(&clock, &mut rng);

    let user_agent = user_agent.map(|ua| ua.to_string());

    let Some(session) = maybe_session else {
        let login = mas_router::Login::and_continue_device_code_grant(grant_id);
        return Ok((cookie_jar, url_builder.redirect(&login)).into_response());
    };

    activity_tracker
        .record_browser_session(&clock, &session)
        .await;

    // TODO: better error handling
    let grant = repo
        .oauth2_device_code_grant()
        .lookup(grant_id)
        .await?
        .context("Device grant not found")
        .map_err(InternalError::from_anyhow)?;

    if grant.expires_at < clock.now() {
        return Err(InternalError::from_anyhow(anyhow::anyhow!(
            "Grant is expired"
        )));
    }

    let client = repo
        .oauth2_client()
        .lookup(grant.client_id)
        .await?
        .context("Client not found")
        .map_err(InternalError::from_anyhow)?;

    let session_counts = count_user_sessions_for_limiting(&mut repo, &session.user).await?;

    // PG_CHANGED: fetch user email before closing the repository
    let email = fetch_user_email(&mut repo, &session.user).await;

    // We can close the repository early, we don't need it at this point
    repo.save().await?;

    // Evaluate the policy
    let res = policy
        .evaluate_authorization_grant(mas_policy::AuthorizationGrantInput {
            grant_type: mas_policy::GrantType::DeviceCode,
            client: &client,
            session_counts: Some(session_counts),
            scope: &grant.scope,
            user: Some(&session.user),
            requester: mas_policy::Requester {
                ip_address: activity_tracker.ip(),
                user_agent,
            },
        })
        .await?;
    if !res.valid() {
        warn!(violation = ?res, "Device code grant for client {} denied by policy", client.id);

        let (csrf_token, cookie_jar) = cookie_jar.csrf_token(&clock, &mut rng);
        let ctx = PolicyViolationContext::for_device_code_grant(grant, client)
            .with_session(session)
            .with_csrf(csrf_token.form_value())
            .with_language(locale);

        let content = templates.render_policy_violation(&ctx)?;

        return Ok((cookie_jar, Html(content)).into_response());
    }

    // Fetch informations about the user. This is purely cosmetic, so we let it
    // fail and put a 1s timeout to it in case we fail to query it
    // XXX: we're likely to need this in other places
    let localpart = &session.user.username;
    let display_name = match tokio::time::timeout(
        Duration::from_secs(1),
        homeserver.query_user(localpart),
    )
    .await
    {
        Ok(Ok(user)) => user.displayname,
        Ok(Err(err)) => {
            tracing::warn!(
                error = &*err as &dyn std::error::Error,
                localpart,
                "Failed to query user"
            );
            None
        }
        Err(_) => {
            tracing::warn!(localpart, "Timed out while querying user");
            None
        }
    };

    let matrix_user = MatrixUser {
        mxid: homeserver.mxid(localpart),
        display_name,
        email
    };

    let ctx = DeviceConsentContext::new(grant, client, matrix_user)
        .with_session(session)
        .with_csrf(csrf_token.form_value())
        .with_language(locale);

    let rendered = templates
        .render_device_consent(&ctx)
        .context("Failed to render template")
        .map_err(InternalError::from_anyhow)?;

    Ok((cookie_jar, Html(rendered)).into_response())
}

#[tracing::instrument(name = "handlers.oauth2.device.consent.post", skip_all)]
pub(crate) async fn post(
    mut rng: BoxRng,
    clock: BoxClock,
    PreferredLanguage(locale): PreferredLanguage,
    State(templates): State<Templates>,
    State(url_builder): State<UrlBuilder>,
    State(homeserver): State<Arc<dyn HomeserverConnection>>,
    mut repo: BoxRepository,
    mut policy: Policy,
    activity_tracker: BoundActivityTracker,
    user_agent: Option<TypedHeader<headers::UserAgent>>,
    cookie_jar: CookieJar,
    Path(grant_id): Path<Ulid>,
    Form(form): Form<ProtectedForm<ConsentForm>>,
) -> Result<Response, InternalError> {
    let form = cookie_jar.verify_form(&clock, form)?;
     // PG_CHANGED: post-logout action
    let post_auth_action = Some(PostAuthAction::continue_device_code_grant(grant_id));
    let (cookie_jar, maybe_session) = match load_session_or_fallback(
        cookie_jar, &clock, &mut rng, &templates, &locale, &mut repo, post_auth_action,
    )
    .await?
    {
        SessionOrFallback::MaybeSession {
            cookie_jar,
            maybe_session,
            ..
        } => (cookie_jar, maybe_session),
        SessionOrFallback::Fallback { response } => return Ok(response),
    };
    let (csrf_token, cookie_jar) = cookie_jar.csrf_token(&clock, &mut rng);

    let user_agent = user_agent.map(|TypedHeader(ua)| ua.to_string());

    let Some(session) = maybe_session else {
        let login = mas_router::Login::and_continue_device_code_grant(grant_id);
        return Ok((cookie_jar, url_builder.redirect(&login)).into_response());
    };

    activity_tracker
        .record_browser_session(&clock, &session)
        .await;

    // TODO: better error handling
    let grant = repo
        .oauth2_device_code_grant()
        .lookup(grant_id)
        .await?
        .context("Device grant not found")
        .map_err(InternalError::from_anyhow)?;

    if grant.expires_at < clock.now() {
        return Err(InternalError::from_anyhow(anyhow::anyhow!(
            "Grant is expired"
        )));
    }

    let client = repo
        .oauth2_client()
        .lookup(grant.client_id)
        .await?
        .context("Client not found")
        .map_err(InternalError::from_anyhow)?;

    let session_counts = count_user_sessions_for_limiting(&mut repo, &session.user).await?;

    // Evaluate the policy
    let res = policy
        .evaluate_authorization_grant(mas_policy::AuthorizationGrantInput {
            grant_type: mas_policy::GrantType::DeviceCode,
            client: &client,
            session_counts: Some(session_counts),
            scope: &grant.scope,
            user: Some(&session.user),
            requester: mas_policy::Requester {
                ip_address: activity_tracker.ip(),
                user_agent,
            },
        })
        .await?;
    if !res.valid() {
        warn!(violation = ?res, "Device code grant for client {} denied by policy", client.id);

        let (csrf_token, cookie_jar) = cookie_jar.csrf_token(&clock, &mut rng);
        let ctx = PolicyViolationContext::for_device_code_grant(grant, client)
            .with_session(session)
            .with_csrf(csrf_token.form_value())
            .with_language(locale);

        let content = templates.render_policy_violation(&ctx)?;

        return Ok((cookie_jar, Html(content)).into_response());
    }

    let grant = if grant.is_pending() {
        match form.action {
            Action::Consent => {
                repo.oauth2_device_code_grant()
                    .fulfill(&clock, grant, &session)
                    .await?
            }
            Action::Reject => {
                repo.oauth2_device_code_grant()
                    .reject(&clock, grant, &session)
                    .await?
            }
        }
    } else {
        // XXX: In case we're not pending, let's just return the grant as-is
        // since it might just be a form resubmission, and feedback is nice enough
        warn!(
            oauth2_device_code.id = %grant.id,
            browser_session.id = %session.id,
            user.id = %session.user.id,
            "Grant is not pending",
        );
        grant
    };

    // PG_CHANGED: if the user gave consent and there's an pg_app_scheme configured in any upstream OAuth provider,
    // fetch the providers before saving the repository (since save consumes self)
    let redirect_url = if matches!(form.action, Action::Consent) && grant.is_fulfilled() {
    // NOTE: While this fetches all providers (unlike `forward_login_hint` which accesses a single provider),
    // in our use case we will always have exactly 1 provider configured, so the performance impact is negligible.
    // A more efficient approach would be to pass the provider ID through the grant/client context if multiple
    // providers needed to be supported in the future.
        let providers = repo.upstream_oauth_provider().all_enabled().await?;
        
        providers
            .iter()
            .find_map(|p| p.pg_app_scheme.as_ref())
            .map(|pg_app_scheme| format!("{}:/qr-login-consent", pg_app_scheme))
    } else {
        None
    };

    // PG_CHANGED: fetch user email before closing the repository
    let email = fetch_user_email(&mut repo, &session.user).await;

    repo.save().await?;

    // PG_CHANGED: if we have a redirect URL, redirect to the deeplink instead of rendering the template
    if let Some(deeplink) = redirect_url {
        return Ok((cookie_jar, Redirect::to(&deeplink)).into_response());
    }
    
    // Fetch informations about the user. This is purely cosmetic, so we let it
    // fail and put a 1s timeout to it in case we fail to query it
    // XXX: we're likely to need this in other places
    let localpart = &session.user.username;
    let display_name = match tokio::time::timeout(
        Duration::from_secs(1),
        homeserver.query_user(localpart),
    )
    .await
    {
        Ok(Ok(user)) => user.displayname,
        Ok(Err(err)) => {
            tracing::warn!(
                error = &*err as &dyn std::error::Error,
                localpart,
                "Failed to query user"
            );
            None
        }
        Err(_) => {
            tracing::warn!(localpart, "Timed out while querying user");
            None
        }
    };

    let matrix_user = MatrixUser {
        mxid: homeserver.mxid(localpart),
        display_name,
        email,
    };

    let ctx = DeviceConsentContext::new(grant, client, matrix_user)
        .with_session(session)
        .with_csrf(csrf_token.form_value())
        .with_language(locale);

    let rendered = templates
        .render_device_consent(&ctx)
        .context("Failed to render template")
        .map_err(InternalError::from_anyhow)?;

    Ok((cookie_jar, Html(rendered)).into_response())
}
