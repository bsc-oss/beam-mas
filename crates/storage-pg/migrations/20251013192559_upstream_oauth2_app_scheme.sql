-- Add the pg_app_scheme column to the upstream_oauth_providers table
ALTER TABLE "upstream_oauth_providers"
    ADD COLUMN "pg_app_scheme" TEXT NULL;
