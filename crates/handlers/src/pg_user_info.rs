//
// Copyright 2026 Belgian Secure Communications (BSC)
//
// SPDX-License-Identifier: AGPL-3.0-only
// Please see LICENSE files in the repository root for full details.
//

use std::sync::Arc;

use mas_matrix::HomeserverConnection;
use mas_storage::BoxRepository;

// PG_CHANGED - displays display name and email instead of username (mxid localpart)
pub async fn fetch_user_email(
    repo: &mut BoxRepository,
    user: &mas_data_model::User,
) -> Option<String> {
    repo
        .user_email()
        .all(user)
        .await
        .ok()
        .and_then(|emails| emails.into_iter().next())
        .map(|e| e.email)
}

// PG_CHANGED - displays display name and email instead of username (mxid localpart)
pub async fn fetch_display_name(
    homeserver: Arc<dyn HomeserverConnection>,
    localpart: &String
) -> Option<String> {
    let display_name = match tokio::time::timeout(
            std::time::Duration::from_secs(1),
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

    return display_name;
}

#[cfg(test)]
mod tests {
    use mas_data_model::clock::MockClock;
    use mas_storage::RepositoryAccess;
    use mas_storage_pg::PgRepository;
    use rand::SeedableRng;
    use rand_chacha::ChaChaRng;
    use sqlx::PgPool;

    use super::*;

    #[sqlx::test(migrator = "mas_storage_pg::MIGRATOR")]
    async fn test_fetch_user_email_returns_email(pool: PgPool) {
        let mut repo = PgRepository::from_pool(&pool).await.unwrap().boxed();
        let mut rng = ChaChaRng::seed_from_u64(42);
        let clock = MockClock::default();

        let user = repo.user().add(&mut rng, &clock, "testuser".to_owned()).await.unwrap();
        repo.user_email()
            .add(&mut rng, &clock, &user, "test@example.com".to_owned())
            .await
            .unwrap();
        repo.save().await.unwrap();

        let mut repo = PgRepository::from_pool(&pool).await.unwrap().boxed();
        let result = fetch_user_email(&mut repo, &user).await;
        assert_eq!(result, Some("test@example.com".to_owned()));
    }

    #[sqlx::test(migrator = "mas_storage_pg::MIGRATOR")]
    async fn test_fetch_user_email_returns_none_when_no_email(pool: PgPool) {
        let mut repo = PgRepository::from_pool(&pool).await.unwrap().boxed();
        let mut rng = ChaChaRng::seed_from_u64(42);
        let clock = MockClock::default();

        let user = repo.user().add(&mut rng, &clock, "nomail".to_owned()).await.unwrap();
        repo.save().await.unwrap();

        let mut repo = PgRepository::from_pool(&pool).await.unwrap().boxed();
        let result = fetch_user_email(&mut repo, &user).await;
        assert_eq!(result, None);
    }
}
