// Copyright 2026 Belgian Secure Communications (BSC)
// Copyright 2024, 2025 New Vector Ltd.
// Copyright 2023, 2024 The Matrix.org Foundation C.I.C.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.
//
// Modified by Belgian Secure Communications for Beam application on 2026-04-30

import { Alert } from "@pg/compound-web";
import type { ReactNode } from "react";
import { Translation } from "react-i18next";

const NotFound: React.FC = () => (
  <Translation>
    {(t): ReactNode => (
      <Alert type="critical" title={t("frontend.not_found_alert_title")} />
    )}
  </Translation>
);

export default NotFound;
