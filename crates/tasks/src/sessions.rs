// Copyright 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

use std::collections::HashSet;

use async_trait::async_trait;
use chrono::Duration;
// PG_CHANGED: pg_scopes, ExpirePgOAuthSessionsJob, ExpirePgSessionsJob
use mas_data_model::pg_scopes;
use mas_storage::{
    compat::CompatSessionFilter,
    oauth2::OAuth2SessionFilter,
    queue::{
        ExpireInactiveCompatSessionsJob, ExpireInactiveOAuthSessionsJob, ExpireInactiveSessionsJob,
        ExpireInactiveUserSessionsJob, ExpirePgOAuthSessionsJob, ExpirePgSessionsJob,
        QueueJobRepositoryExt, SyncDevicesJob,
    },
    user::BrowserSessionFilter,
};

use crate::{
    State,
    new_queue::{JobContext, JobError, RunnableJob},
};

#[async_trait]
impl RunnableJob for ExpireInactiveSessionsJob {
    async fn run(&self, state: &State, _context: JobContext) -> Result<(), JobError> {
        let Some(config) = state.site_config().session_expiration.as_ref() else {
            // Automatic session expiration is disabled
            return Ok(());
        };

        let clock = state.clock();
        let mut rng = state.rng();
        let now = clock.now();
        let mut repo = state.repository().await.map_err(JobError::retry)?;

        if let Some(ttl) = config.oauth_session_inactivity_ttl {
            repo.queue_job()
                .schedule_job(
                    &mut rng,
                    clock,
                    ExpireInactiveOAuthSessionsJob::new(now - ttl),
                )
                .await
                .map_err(JobError::retry)?;
        }

        if let Some(ttl) = config.compat_session_inactivity_ttl {
            repo.queue_job()
                .schedule_job(
                    &mut rng,
                    clock,
                    ExpireInactiveCompatSessionsJob::new(now - ttl),
                )
                .await
                .map_err(JobError::retry)?;
        }

        if let Some(ttl) = config.user_session_inactivity_ttl {
            repo.queue_job()
                .schedule_job(
                    &mut rng,
                    clock,
                    ExpireInactiveUserSessionsJob::new(now - ttl),
                )
                .await
                .map_err(JobError::retry)?;
        }

        repo.save().await.map_err(JobError::retry)?;

        Ok(())
    }
}

#[async_trait]
impl RunnableJob for ExpireInactiveOAuthSessionsJob {
    async fn run(&self, state: &State, _context: JobContext) -> Result<(), JobError> {
        let mut repo = state.repository().await.map_err(JobError::retry)?;
        let clock = state.clock();
        let mut rng = state.rng();
        let mut users_synced = HashSet::new();

        // This delay is used to space out the device sync jobs
        // We add 10 seconds between each device sync, meaning that it will spread out
        // the syncs over ~16 minutes max if we get a full batch of 100 users
        let mut delay = Duration::minutes(1);

        let filter = OAuth2SessionFilter::new()
            .with_last_active_before(self.threshold())
            .for_any_user()
            .only_dynamic_clients()
            .active_only();

        let pagination = self.pagination(100);

        let page = repo
            .oauth2_session()
            .list(filter, pagination)
            .await
            .map_err(JobError::retry)?;

        if let Some(job) = self.next(&page) {
            tracing::info!("Scheduling job to expire the next batch of inactive sessions");
            repo.queue_job()
                .schedule_job(&mut rng, clock, job)
                .await
                .map_err(JobError::retry)?;
        }

        for edge in page.edges {
            if let Some(user_id) = edge.node.user_id {
                let inserted = users_synced.insert(user_id);
                if inserted {
                    tracing::info!(user.id = %user_id, "Scheduling devices sync for user");
                    repo.queue_job()
                        .schedule_job_later(
                            &mut rng,
                            clock,
                            SyncDevicesJob::new_for_id(user_id),
                            clock.now() + delay,
                        )
                        .await
                        .map_err(JobError::retry)?;
                    delay += Duration::seconds(10);
                }
            }

            repo.oauth2_session()
                .finish(clock, edge.node)
                .await
                .map_err(JobError::retry)?;
        }

        repo.save().await.map_err(JobError::retry)?;

        Ok(())
    }
}

#[async_trait]
impl RunnableJob for ExpireInactiveCompatSessionsJob {
    async fn run(&self, state: &State, _context: JobContext) -> Result<(), JobError> {
        let mut repo = state.repository().await.map_err(JobError::retry)?;
        let clock = state.clock();
        let mut rng = state.rng();
        let mut users_synced = HashSet::new();

        // This delay is used to space out the device sync jobs
        // We add 10 seconds between each device sync, meaning that it will spread out
        // the syncs over ~16 minutes max if we get a full batch of 100 users
        let mut delay = Duration::minutes(1);

        let filter = CompatSessionFilter::new()
            .with_last_active_before(self.threshold())
            .active_only();

        let pagination = self.pagination(100);

        let page = repo
            .compat_session()
            .list(filter, pagination)
            .await
            .map_err(JobError::retry)?
            .map(|(c, _)| c);

        if let Some(job) = self.next(&page) {
            tracing::info!("Scheduling job to expire the next batch of inactive sessions");
            repo.queue_job()
                .schedule_job(&mut rng, clock, job)
                .await
                .map_err(JobError::retry)?;
        }

        for edge in page.edges {
            let inserted = users_synced.insert(edge.node.user_id);
            if inserted {
                tracing::info!(user.id = %edge.node.user_id, "Scheduling devices sync for user");
                repo.queue_job()
                    .schedule_job_later(
                        &mut rng,
                        clock,
                        SyncDevicesJob::new_for_id(edge.node.user_id),
                        clock.now() + delay,
                    )
                    .await
                    .map_err(JobError::retry)?;
                delay += Duration::seconds(10);
            }

            repo.compat_session()
                .finish(clock, edge.node)
                .await
                .map_err(JobError::retry)?;
        }

        repo.save().await.map_err(JobError::retry)?;

        Ok(())
    }
}

#[async_trait]
impl RunnableJob for ExpireInactiveUserSessionsJob {
    async fn run(&self, state: &State, _context: JobContext) -> Result<(), JobError> {
        let mut repo = state.repository().await.map_err(JobError::retry)?;
        let clock = state.clock();
        let mut rng = state.rng();

        let filter = BrowserSessionFilter::new()
            .with_last_active_before(self.threshold())
            .active_only();

        let pagination = self.pagination(100);

        let page = repo
            .browser_session()
            .list(filter, pagination)
            .await
            .map_err(JobError::retry)?;

        if let Some(job) = self.next(&page) {
            tracing::info!("Scheduling job to expire the next batch of inactive sessions");
            repo.queue_job()
                .schedule_job(&mut rng, clock, job)
                .await
                .map_err(JobError::retry)?;
        }

        for edge in page.edges {
            repo.browser_session()
                .finish(clock, edge.node)
                .await
                .map_err(JobError::retry)?;
        }

        repo.save().await.map_err(JobError::retry)?;

        Ok(())
    }
}

// PG_CHANGED
#[async_trait]
impl RunnableJob for ExpirePgSessionsJob {
    async fn run(&self, state: &State, _context: JobContext) -> Result<(), JobError> {
        let Some(config) = state.site_config().pg_session_expiration.as_ref() else {
            // PG session expiration is not configured
            return Ok(());
        };

        if !config.enabled {
            // PG session expiration is disabled
            return Ok(());
        }

        let clock = state.clock();
        let mut rng = state.rng();
        let now = clock.now();
        let mut repo = state.repository().await.map_err(JobError::retry)?;

        // Calculate the thresholds for each scope type
        let short_lived_created_before = now - config.short_lived_max_lifetime;
        let long_lived_created_before = now - config.long_lived_max_lifetime;
        let inactive_before = now - config.inactivity_ttl;

        repo.queue_job()
            .schedule_job(
                &mut rng,
                clock,
                ExpirePgOAuthSessionsJob::new(
                    short_lived_created_before,
                    long_lived_created_before,
                    inactive_before,
                ),
            )
            .await
            .map_err(JobError::retry)?;

        repo.save().await.map_err(JobError::retry)?;

        Ok(())
    }
}

#[async_trait]
impl RunnableJob for ExpirePgOAuthSessionsJob {
    async fn run(&self, state: &State, _context: JobContext) -> Result<(), JobError> {
        let mut repo = state.repository().await.map_err(JobError::retry)?;
        let clock = state.clock();
        let mut rng = state.rng();
        let mut users_synced = HashSet::new();

        // This delay is used to space out the device sync jobs
        // We add 10 seconds between each device sync, meaning that it will spread out
        // the syncs over ~16 minutes max if we get a full batch of 100 users
        let mut delay = Duration::minutes(1);
        // Extra explenation about this, why it is needed:
        // The SyncDevicesJob is scheduled to notify the Matrix homeserver that a device (session) has been removed/expired.
        // When a user logs into Matrix via MAS, MAS creates a "device" on the homeserver (e.g., Synapse). When a session is expired/revoked, MAS needs to tell the homeserver to remove that device so:
        // The device disappears from the user's device list
        // The homeserver can clean up E2EE keys associated with that device
        // Other users see the device as gone (for verification purposes)
        // Stagger this to avoid unnecesary load on the homeserver


        // We need to find sessions that:
        // 1. Are active
        // 2. Have been inactive for at least the inactivity TTL
        // 3. Have one of the PG lifetime scope tokens
        // 4. Were created before their respective lifetime threshold
        //
        // We filter by the short-lived threshold (10h) since it's less restrictive
        // than long-lived (30d). Any session that could be expired must be at least
        // 10 hours old. We then check the exact scope and created_at in code.
        let pg_scopes: &[&str] = &[
            &pg_scopes::PG_SESSION_SHORT_LIVED,
            &pg_scopes::PG_SESSION_LONG_LIVED,
        ];
        let filter = OAuth2SessionFilter::new()
            .with_last_active_before(self.inactive_before())
            .with_created_before(self.short_lived_created_before())
            .with_scope_containing_any(pg_scopes)
            .for_any_user()
            .active_only();

        let pagination = self.pagination(100);

        let page = repo
            .oauth2_session()
            .list(filter, pagination)
            .await
            .map_err(JobError::retry)?;

        if let Some(job) = self.next(&page) {
            tracing::info!(
                "Scheduling job to expire the next batch of PG sessions"
            );
            repo.queue_job()
                .schedule_job(&mut rng, clock, job)
                .await
                .map_err(JobError::retry)?;
        }

        for edge in page.edges {
            // Check if this session has a PG lifetime and if it's expired
            let should_expire = if edge.node.scope.contains(&pg_scopes::PG_SESSION_SHORT_LIVED) {
                // Short-lived sessions expire after short_lived_max_lifetime
                edge.node.created_at < self.short_lived_created_before()
            } else if edge.node.scope.contains(&pg_scopes::PG_SESSION_LONG_LIVED) {
                // Long-lived sessions expire after long_lived_max_lifetime
                edge.node.created_at < self.long_lived_created_before()
            } else {
                // Sessions without either scope are not affected
                false
            };

            if !should_expire {
                continue;
            }

            tracing::info!(
                session.id = %edge.node.id,
                session.created_at = %edge.node.created_at,
                has_short_lived_scope = edge.node.scope.contains(&pg_scopes::PG_SESSION_SHORT_LIVED),
                has_long_lived_scope = edge.node.scope.contains(&pg_scopes::PG_SESSION_LONG_LIVED),
                "Expiring PG OAuth2 session"
            );

            // Here we notify Synapse to also remove the device/session
            if let Some(user_id) = edge.node.user_id {
                let inserted = users_synced.insert(user_id);
                if inserted {
                    tracing::info!(user.id = %user_id, "Scheduling devices sync for user");
                    repo.queue_job()
                        .schedule_job_later(
                            &mut rng,
                            clock,
                            SyncDevicesJob::new_for_id(user_id),
                            clock.now() + delay,
                        )
                        .await
                        .map_err(JobError::retry)?;
                    delay += Duration::seconds(10);
                }
            }

            repo.oauth2_session()
                .finish(clock, edge.node)
                .await
                .map_err(JobError::retry)?;
        }

        repo.save().await.map_err(JobError::retry)?;

        Ok(())
    }
}
