// Copyright 2026 Belgian Secure Communications (BSC)
// Copyright 2024, 2025 New Vector Ltd.
// Copyright 2022-2024 The Matrix.org Foundation C.I.C.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.
//
// Modified by Belgian Secure Communications for Beam application on 2026-04-30

type AppConfig = {
  root: string;
  graphqlEndpoint: string;
  // PG_CHANGED
  csrfToken?: string;
};

interface IWindow {
  APP_CONFIG?: AppConfig;
}

const config: AppConfig = (typeof window !== "undefined" &&
  (window as IWindow).APP_CONFIG) || {
  root: "/",
  graphqlEndpoint: "/graphql",
  // PG_CHANGED
  csrfToken: undefined,
};

export default config;
