// TODO: PG License

//! Tests for background jobs, particularly PG session expiration and post_auth_action preservation.

#[cfg(test)]
mod tests {
    use chrono::Duration;
    use hyper::{Request, StatusCode};
    use mas_data_model::{pg_scopes, Clock, PgSessionExpirationConfig, SiteConfig};
    use mas_router::SimpleRoute;
    use oauth2_types::registration::ClientRegistrationResponse;
    use oauth2_types::scope::{Scope, OPENID};
    use sqlx::PgPool;

    use crate::test_utils::{setup, test_site_config, CookieHelper, RequestBuilderExt, ResponseExt, TestState};
    use crate::oauth2::generate_token_pair;
    use mas_storage::queue::{ExpirePgSessionsJob, QueueJobRepositoryExt as _};

    /// Helper function to create a site config with PG session expiration enabled
    fn site_config_with_pg_session_expiration() -> SiteConfig {
        let mut config = test_site_config();
        config.pg_session_expiration = Some(PgSessionExpirationConfig {
            enabled: true,
            // Short-lived sessions expire after 10 hours
            short_lived_max_lifetime: Duration::try_hours(10).unwrap(),
            // Long-lived sessions expire after 30 days
            long_lived_max_lifetime: Duration::try_days(30).unwrap(),
            // Sessions must be inactive for 1 hour before expiration
            inactivity_ttl: Duration::try_hours(1).unwrap(),
        });
        config
    }

    /// PG_CHANGED: Test that OAuth2 sessions with the short-lived scope are expired
    /// after exceeding their maximum lifetime AND being inactive.
    #[sqlx::test(migrator = "mas_storage_pg::MIGRATOR")]
    async fn test_expire_short_lived_sessions(pool: PgPool) {
        setup();
        let state = TestState::from_pool_with_site_config(
            pool.clone(),
            site_config_with_pg_session_expiration(),
        )
        .await
        .unwrap();

        // Register a client
        let request =
            Request::post(mas_router::OAuth2RegistrationEndpoint::PATH).json(serde_json::json!({
                "client_uri": "https://example.com/",
                "redirect_uris": ["https://example.com/callback"],
                "token_endpoint_auth_method": "client_secret_post",
                "response_types": ["code"],
                "grant_types": ["authorization_code", "refresh_token"],
            }));

        let response = state.request(request).await;
        response.assert_status(StatusCode::CREATED);

        let client_registration: ClientRegistrationResponse = response.json();
        let client_id = client_registration.client_id;

        // Create a user and browser session
        let mut repo = state.repository().await.unwrap();

        let user = repo
            .user()
            .add(&mut state.rng(), &state.clock, "alice".to_owned())
            .await
            .unwrap();

        let browser_session = repo
            .browser_session()
            .add(&mut state.rng(), &state.clock, &user, None)
            .await
            .unwrap();

        // Lookup the client
        let client = repo
            .oauth2_client()
            .find_by_client_id(&client_id)
            .await
            .unwrap()
            .unwrap();

        // Create OAuth2 session WITH the short-lived scope
        let session_short_lived = repo
            .oauth2_session()
            .add_from_browser_session(
                &mut state.rng(),
                &state.clock,
                &client,
                &browser_session,
                Scope::from_iter([OPENID, pg_scopes::PG_SESSION_SHORT_LIVED]),
            )
            .await
            .unwrap();

        // Create OAuth2 session WITHOUT any PG scope (should NOT be expired)
        let session_no_pg_scope = repo
            .oauth2_session()
            .add_from_browser_session(
                &mut state.rng(),
                &state.clock,
                &client,
                &browser_session,
                Scope::from_iter([OPENID]),
            )
            .await
            .unwrap();

        let session_short_lived_id = session_short_lived.id;
        let session_no_pg_scope_id = session_no_pg_scope.id;

        // Record initial activity for both sessions (sets last_active_at)
        // This is important because the expire job filters by last_active_at
        repo.oauth2_session()
            .record_batch_activity(vec![
                (session_short_lived_id, state.clock.now(), None),
                (session_no_pg_scope_id, state.clock.now(), None),
            ])
            .await
            .unwrap();

        // Generate tokens for both sessions
        let (_access_token_short_lived, _refresh_token_short_lived) = generate_token_pair(
            &mut state.rng(),
            &state.clock,
            &mut repo,
            &session_short_lived,
            Duration::microseconds(5 * 60 * 1000 * 1000),
        )
        .await
        .unwrap();

        let (_access_token_no_pg_scope, _refresh_token_no_pg_scope) = generate_token_pair(
            &mut state.rng(),
            &state.clock,
            &mut repo,
            &session_no_pg_scope,
            Duration::microseconds(5 * 60 * 1000 * 1000),
        )
        .await
        .unwrap();

        repo.save().await.unwrap();

        // Verify both sessions are active
        let mut repo = state.repository().await.unwrap();
        let session_short_lived = repo
            .oauth2_session()
            .lookup(session_short_lived_id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            session_short_lived.finished_at().is_none(),
            "Short-lived session should be active"
        );

        let session_no_pg_scope = repo
            .oauth2_session()
            .lookup(session_no_pg_scope_id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            session_no_pg_scope.finished_at().is_none(),
            "No PG scope session should be active"
        );
        drop(repo);

        // Advance the clock past the short-lived max lifetime (10h) + inactivity TTL (1h)
        // Total: 11 hours
        state.clock.advance(Duration::try_hours(11).unwrap());

        // Schedule the expire job manually
        let mut repo = state.repository().await.unwrap();
        repo.queue_job()
            .schedule_job(&mut state.rng(), &state.clock, ExpirePgSessionsJob)
            .await
            .unwrap();
        repo.save().await.unwrap();

        // Run all jobs in queue (first run processes ExpirePgSessionsJob which schedules ExpirePgOAuthSessionsJob)
        state.run_jobs_in_queue().await;
        // Second run processes ExpirePgOAuthSessionsJob which actually expires the sessions
        state.run_jobs_in_queue().await;

        // Verify the short-lived session is now expired
        let mut repo = state.repository().await.unwrap();
        let session_short_lived = repo
            .oauth2_session()
            .lookup(session_short_lived_id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            session_short_lived.finished_at().is_some(),
            "Short-lived session should be expired after exceeding lifetime + inactivity"
        );

        // Verify the session without PG scope is still active
        let session_no_pg_scope = repo
            .oauth2_session()
            .lookup(session_no_pg_scope_id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            session_no_pg_scope.finished_at().is_none(),
            "Session without PG scope should still be active"
        );
    }

    /// PG_CHANGED: Test that OAuth2 sessions with the long-lived scope are expired
    /// after exceeding their maximum lifetime AND being inactive.
    #[sqlx::test(migrator = "mas_storage_pg::MIGRATOR")]
    async fn test_expire_long_lived_sessions(pool: PgPool) {
        setup();
        let state = TestState::from_pool_with_site_config(
            pool.clone(),
            site_config_with_pg_session_expiration(),
        )
        .await
        .unwrap();

        // Register a client
        let request =
            Request::post(mas_router::OAuth2RegistrationEndpoint::PATH).json(serde_json::json!({
                "client_uri": "https://example.com/",
                "redirect_uris": ["https://example.com/callback"],
                "token_endpoint_auth_method": "client_secret_post",
                "response_types": ["code"],
                "grant_types": ["authorization_code", "refresh_token"],
            }));

        let response = state.request(request).await;
        response.assert_status(StatusCode::CREATED);

        let client_registration: ClientRegistrationResponse = response.json();
        let client_id = client_registration.client_id;

        // Create a user and browser session
        let mut repo = state.repository().await.unwrap();

        let user = repo
            .user()
            .add(&mut state.rng(), &state.clock, "bob".to_owned())
            .await
            .unwrap();

        let browser_session = repo
            .browser_session()
            .add(&mut state.rng(), &state.clock, &user, None)
            .await
            .unwrap();

        // Lookup the client
        let client = repo
            .oauth2_client()
            .find_by_client_id(&client_id)
            .await
            .unwrap()
            .unwrap();

        // Create OAuth2 session WITH the long-lived scope
        let session_long_lived = repo
            .oauth2_session()
            .add_from_browser_session(
                &mut state.rng(),
                &state.clock,
                &client,
                &browser_session,
                Scope::from_iter([OPENID, pg_scopes::PG_SESSION_LONG_LIVED]),
            )
            .await
            .unwrap();

        let session_long_lived_id = session_long_lived.id;

        // Record initial activity for the session (sets last_active_at)
        repo.oauth2_session()
            .record_batch_activity(vec![(session_long_lived_id, state.clock.now(), None)])
            .await
            .unwrap();

        // Generate tokens
        let (_access_token, _refresh_token) = generate_token_pair(
            &mut state.rng(),
            &state.clock,
            &mut repo,
            &session_long_lived,
            Duration::microseconds(5 * 60 * 1000 * 1000),
        )
        .await
        .unwrap();

        repo.save().await.unwrap();

        // Verify session is active
        let mut repo = state.repository().await.unwrap();
        let session_long_lived = repo
            .oauth2_session()
            .lookup(session_long_lived_id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            session_long_lived.finished_at().is_none(),
            "Long-lived session should be active"
        );
        drop(repo);

        // Advance the clock past the long-lived max lifetime (30d) + inactivity TTL (1h)
        // Total: 30 days + 1 hour
        state
            .clock
            .advance(Duration::try_days(30).unwrap() + Duration::try_hours(1).unwrap());

        // Schedule the expire job manually
        let mut repo = state.repository().await.unwrap();
        repo.queue_job()
            .schedule_job(&mut state.rng(), &state.clock, ExpirePgSessionsJob)
            .await
            .unwrap();
        repo.save().await.unwrap();

        // Run all jobs in queue (first run processes ExpirePgSessionsJob which schedules ExpirePgOAuthSessionsJob)
        state.run_jobs_in_queue().await;
        // Second run processes ExpirePgOAuthSessionsJob which actually expires the sessions
        state.run_jobs_in_queue().await;

        // Verify the long-lived session is now expired
        let mut repo = state.repository().await.unwrap();
        let session_long_lived = repo
            .oauth2_session()
            .lookup(session_long_lived_id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            session_long_lived.finished_at().is_some(),
            "Long-lived session should be expired after exceeding lifetime + inactivity"
        );
    }

    /// PG_CHANGED: Test that sessions are NOT expired if they are still active
    /// (even if they exceed their lifetime).
    #[sqlx::test(migrator = "mas_storage_pg::MIGRATOR")]
    async fn test_active_sessions_not_expired(pool: PgPool) {
        setup();
        let state = TestState::from_pool_with_site_config(
            pool.clone(),
            site_config_with_pg_session_expiration(),
        )
        .await
        .unwrap();

        // Register a client
        let request =
            Request::post(mas_router::OAuth2RegistrationEndpoint::PATH).json(serde_json::json!({
                "client_uri": "https://example.com/",
                "redirect_uris": ["https://example.com/callback"],
                "token_endpoint_auth_method": "client_secret_post",
                "response_types": ["code"],
                "grant_types": ["authorization_code", "refresh_token"],
            }));

        let response = state.request(request).await;
        response.assert_status(StatusCode::CREATED);

        let client_registration: ClientRegistrationResponse = response.json();
        let client_id = client_registration.client_id;

        // Create a user and browser session
        let mut repo = state.repository().await.unwrap();

        let user = repo
            .user()
            .add(&mut state.rng(), &state.clock, "charlie".to_owned())
            .await
            .unwrap();

        let browser_session = repo
            .browser_session()
            .add(&mut state.rng(), &state.clock, &user, None)
            .await
            .unwrap();

        // Lookup the client
        let client = repo
            .oauth2_client()
            .find_by_client_id(&client_id)
            .await
            .unwrap()
            .unwrap();

        // Create OAuth2 session WITH the short-lived scope
        let session_short_lived = repo
            .oauth2_session()
            .add_from_browser_session(
                &mut state.rng(),
                &state.clock,
                &client,
                &browser_session,
                Scope::from_iter([OPENID, pg_scopes::PG_SESSION_SHORT_LIVED]),
            )
            .await
            .unwrap();

        let session_short_lived_id = session_short_lived.id;

        // Record initial activity for the session (sets last_active_at)
        repo.oauth2_session()
            .record_batch_activity(vec![(session_short_lived_id, state.clock.now(), None)])
            .await
            .unwrap();

        // Generate tokens
        let (_access_token, _refresh_token) = generate_token_pair(
            &mut state.rng(),
            &state.clock,
            &mut repo,
            &session_short_lived,
            Duration::microseconds(5 * 60 * 1000 * 1000),
        )
        .await
        .unwrap();

        repo.save().await.unwrap();

        // Advance the clock past the short-lived max lifetime (10h) but NOT past inactivity TTL
        // Advance 10 hours (exceeds lifetime) but then update last_active
        state.clock.advance(Duration::try_hours(10).unwrap());

        // Update the session's last_active to simulate recent activity
        let mut repo = state.repository().await.unwrap();
        repo.oauth2_session()
            .record_batch_activity(vec![(session_short_lived_id, state.clock.now(), None)])
            .await
            .unwrap();
        repo.save().await.unwrap();

        // Now advance past the lifetime threshold (additional hour to be safe)
        state.clock.advance(Duration::try_hours(1).unwrap());

        // Schedule the expire job manually
        let mut repo = state.repository().await.unwrap();
        repo.queue_job()
            .schedule_job(&mut state.rng(), &state.clock, ExpirePgSessionsJob)
            .await
            .unwrap();
        repo.save().await.unwrap();

        // Run all jobs in queue (first run processes ExpirePgSessionsJob which schedules ExpirePgOAuthSessionsJob)
        state.run_jobs_in_queue().await;
        // Second run processes ExpirePgOAuthSessionsJob
        state.run_jobs_in_queue().await;

        // Verify the short-lived session is still active because it was recently active
        let mut repo = state.repository().await.unwrap();
        let session_short_lived = repo
            .oauth2_session()
            .lookup(session_short_lived_id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            session_short_lived.finished_at().is_none(),
            "Session should NOT be expired because it was recently active"
        );
    }

    /// PG_CHANGED: Test that long-lived sessions are NOT expired when they haven't
    /// exceeded their lifetime, even if they've been inactive.
    #[sqlx::test(migrator = "mas_storage_pg::MIGRATOR")]
    async fn test_long_lived_not_expired_before_lifetime(pool: PgPool) {
        setup();
        let state = TestState::from_pool_with_site_config(
            pool.clone(),
            site_config_with_pg_session_expiration(),
        )
        .await
        .unwrap();

        // Register a client
        let request =
            Request::post(mas_router::OAuth2RegistrationEndpoint::PATH).json(serde_json::json!({
                "client_uri": "https://example.com/",
                "redirect_uris": ["https://example.com/callback"],
                "token_endpoint_auth_method": "client_secret_post",
                "response_types": ["code"],
                "grant_types": ["authorization_code", "refresh_token"],
            }));

        let response = state.request(request).await;
        response.assert_status(StatusCode::CREATED);

        let client_registration: ClientRegistrationResponse = response.json();
        let client_id = client_registration.client_id;

        // Create a user and browser session
        let mut repo = state.repository().await.unwrap();

        let user = repo
            .user()
            .add(&mut state.rng(), &state.clock, "dave".to_owned())
            .await
            .unwrap();

        let browser_session = repo
            .browser_session()
            .add(&mut state.rng(), &state.clock, &user, None)
            .await
            .unwrap();

        // Lookup the client
        let client = repo
            .oauth2_client()
            .find_by_client_id(&client_id)
            .await
            .unwrap()
            .unwrap();

        // Create OAuth2 session WITH the long-lived scope
        let session_long_lived = repo
            .oauth2_session()
            .add_from_browser_session(
                &mut state.rng(),
                &state.clock,
                &client,
                &browser_session,
                Scope::from_iter([OPENID, pg_scopes::PG_SESSION_LONG_LIVED]),
            )
            .await
            .unwrap();

        let session_long_lived_id = session_long_lived.id;

        // Record initial activity for the session (sets last_active_at)
        repo.oauth2_session()
            .record_batch_activity(vec![(session_long_lived_id, state.clock.now(), None)])
            .await
            .unwrap();

        // Generate tokens
        let (_access_token, _refresh_token) = generate_token_pair(
            &mut state.rng(),
            &state.clock,
            &mut repo,
            &session_long_lived,
            Duration::microseconds(5 * 60 * 1000 * 1000),
        )
        .await
        .unwrap();

        repo.save().await.unwrap();

        // Advance the clock past the short-lived max lifetime but NOT past long-lived
        // Short-lived is 10h, long-lived is 30d
        // Advance 15 days (would expire short-lived but not long-lived)
        state.clock.advance(Duration::try_days(15).unwrap());

        // Schedule the expire job manually
        let mut repo = state.repository().await.unwrap();
        repo.queue_job()
            .schedule_job(&mut state.rng(), &state.clock, ExpirePgSessionsJob)
            .await
            .unwrap();
        repo.save().await.unwrap();

        // Run all jobs in queue (first run processes ExpirePgSessionsJob which schedules ExpirePgOAuthSessionsJob)
        state.run_jobs_in_queue().await;
        // Second run processes ExpirePgOAuthSessionsJob
        state.run_jobs_in_queue().await;

        // Verify the long-lived session is still active
        let mut repo = state.repository().await.unwrap();
        let session_long_lived = repo
            .oauth2_session()
            .lookup(session_long_lived_id)
            .await
            .unwrap()
            .unwrap();
        assert!(
            session_long_lived.finished_at().is_none(),
            "Long-lived session should NOT be expired before 30 days"
        );
    }

    /// PG_CHANGED: Test that when a user's browser session is finished (remote logout),
    /// visiting the login page with a post_auth_action shows the logged_out page
    /// with the action preserved in the logout form.
    #[sqlx::test(migrator = "mas_storage_pg::MIGRATOR")]
    async fn test_logged_out_page_preserves_post_auth_action(pool: PgPool) {
        setup();
        let state = TestState::from_pool(pool.clone()).await.unwrap();
        let mut rng = state.rng();
        let cookies = CookieHelper::new();

        // Create a user with a browser session
        let mut repo = state.repository().await.unwrap();
        let user = repo
            .user()
            .add(&mut rng, &state.clock, "john".to_owned())
            .await
            .unwrap();

        // Create a browser session
        let browser_session = repo
            .browser_session()
            .add(&mut rng, &state.clock, &user, None)
            .await
            .unwrap();

        // Set up the session cookie BEFORE finishing the session
        // (simulates the user having a valid cookie but session was ended remotely)
        use mas_axum_utils::SessionInfoExt as _;
        let cookie_jar = state.cookie_jar().set_session(&browser_session);
        cookies.import(cookie_jar);

        // Finish the session (simulate remote logout)
        repo.browser_session()
            .finish(&state.clock, browser_session)
            .await
            .unwrap();
        repo.save().await.unwrap();

        // Create a fake authorization grant ID for the post_auth_action
        let grant_id = ulid::Ulid::new();

        // Visit login page with post_auth_action
        let request = Request::get(&format!(
            "/login?kind=continue_authorization_grant&id={grant_id}"
        ))
        .empty();
        let request = cookies.with_cookies(request);
        let response = state.request(request).await;
        cookies.save_cookies(&response);

        // Should get 200 OK with the logged_out page
        response.assert_status(StatusCode::OK);
        response.assert_header_value(hyper::header::CONTENT_TYPE, "text/html; charset=utf-8");

        let body = response.body();
        
        // The page should contain the "Session terminated" message (from mas.account.logged_out.heading)
        assert!(
            body.contains("Session terminated") || body.contains("logged out") || body.contains("Logged out"),
            "Response should show the session terminated page, got: {}", &body[..body.len().min(500)]
        );
        // The page should contain a logout form with the post_auth_action fields
        assert!(
            body.contains("name=\"kind\""),
            "Logout form should contain the post_auth_action 'kind' field"
        );
        assert!(
            body.contains("continue_authorization_grant"),
            "Logout form should contain the action kind value"
        );
        assert!(
            body.contains(&grant_id.to_string()),
            "Logout form should contain the grant ID"
        );
    }

    /// PG_CHANGED: Test that after clicking logout on the logged_out page,
    /// the user is redirected to the login page with the post_auth_action preserved.
    #[sqlx::test(migrator = "mas_storage_pg::MIGRATOR")]
    async fn test_logout_preserves_post_auth_action_redirect(pool: PgPool) {
        setup();
        let state = TestState::from_pool(pool.clone()).await.unwrap();
        let mut rng = state.rng();
        let cookies = CookieHelper::new();

        // Create a user with a browser session
        let mut repo = state.repository().await.unwrap();
        let user = repo
            .user()
            .add(&mut rng, &state.clock, "john".to_owned())
            .await
            .unwrap();

        // Create a browser session
        let browser_session = repo
            .browser_session()
            .add(&mut rng, &state.clock, &user, None)
            .await
            .unwrap();

        // Set up the session cookie BEFORE finishing the session
        // (simulates the user having a valid cookie but session was ended remotely)
        use mas_axum_utils::SessionInfoExt as _;
        let cookie_jar = state.cookie_jar().set_session(&browser_session);
        cookies.import(cookie_jar);

        // Finish the session (simulate remote logout)
        repo.browser_session()
            .finish(&state.clock, browser_session)
            .await
            .unwrap();
        repo.save().await.unwrap();

        // Create a fake authorization grant ID for the post_auth_action
        let grant_id = ulid::Ulid::new();

        // Visit login page with post_auth_action to get CSRF token
        let request = Request::get(&format!(
            "/login?kind=continue_authorization_grant&id={grant_id}"
        ))
        .empty();
        let request = cookies.with_cookies(request);
        let response = state.request(request).await;
        cookies.save_cookies(&response);

        response.assert_status(StatusCode::OK);

        // Extract the CSRF token from the response body
        let csrf_token = response
            .body()
            .split("name=\"csrf\" value=\"")
            .nth(1)
            .unwrap()
            .split('\"')
            .next()
            .unwrap();

        // Submit the logout form with the post_auth_action
        let request = Request::post("/logout").form(serde_json::json!({
            "csrf": csrf_token,
            "kind": "continue_authorization_grant",
            "id": grant_id.to_string(),
        }));
        let request = cookies.with_cookies(request);
        let response = state.request(request).await;
        cookies.save_cookies(&response);

        // Should get a redirect
        response.assert_status(StatusCode::SEE_OTHER);

        // The redirect location should contain the post_auth_action
        let location = response
            .headers()
            .get("location")
            .expect("Should have Location header")
            .to_str()
            .unwrap();

        // The redirect should go to the consent page (where go_next() redirects for ContinueAuthorizationGrant)
        assert!(
            location.contains(&grant_id.to_string()),
            "Redirect location should contain the grant ID: {location}"
        );
    }
}
