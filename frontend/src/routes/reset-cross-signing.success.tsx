// Copyright 2026 Belgian Secure Communications (BSC)
// Copyright 2024, 2025 New Vector Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.
//
// Modified by Belgian Secure Communications for Beam application on 2026-04-30

import IconCheckCircleSolid from "@pg/compound-design-tokens/assets/web/icons/check-circle-solid";
import { Text } from "@pg/compound-web";
import { useTranslation } from "react-i18next";
import PageHeading from "../components/PageHeading";

// This value comes from Synapse and we have no way to query it from here
// https://github.com/element-hq/synapse/blob/34b758644611721911a223814a7b35d8e14067e6/synapse/rest/admin/users.py#L1335
const CROSS_SIGNING_REPLACEMENT_PERIOD_MS = 10 * 60 * 1000; // 10 minutes

export const Route = createFileRoute({
  component: () => {
    const { t } = useTranslation();
    return (
      <>
        <PageHeading
          Icon={IconCheckCircleSolid}
          title={t("frontend.reset_cross_signing.success.heading")}
          success
        />
        <Text className="text-center text-secondary" size="md">
          {t("frontend.reset_cross_signing.success.description", {
            minutes: CROSS_SIGNING_REPLACEMENT_PERIOD_MS / (60 * 1000),
          })}
        </Text>
      </>
    );
  },
});
