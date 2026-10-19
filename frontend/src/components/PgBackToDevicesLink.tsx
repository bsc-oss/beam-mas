// Copyright 2026 Belgian Secure Communications (BSC)
//
// SPDX-License-Identifier: AGPL-3.0-only
// Please see LICENSE files in the repository root for full details.

import IconArrowLeft from "@vector-im/compound-design-tokens/assets/web/icons/arrow-left";
import { useTranslation } from "react-i18next";
import { ButtonLink } from "./ButtonLink";
import styles from "./PgBackToDevicesLink.module.css";

/**
 * Way back to the device list. The browsers list is only reachable from there, and
 * otherwise leaves people stranded.
 */
const PgBackToDevicesLink: React.FC = () => {
  const { t } = useTranslation();

  return (
    <ButtonLink
      Icon={IconArrowLeft}
      kind="tertiary"
      to="/sessions"
      className={styles.backLink}
    >
      {t("frontend.pg_back_to_devices")}
    </ButtonLink>
  );
};

export default PgBackToDevicesLink;
