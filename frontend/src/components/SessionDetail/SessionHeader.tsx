// Copyright 2024, 2025 New Vector Ltd.
// Copyright 2023, 2024 The Matrix.org Foundation C.I.C.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

import IconChevronLeft from "@pg/compound-design-tokens/assets/web/icons/chevron-left";
import { H3 } from "@pg/compound-web";
import { Link } from "@tanstack/react-router";

import styles from "./SessionHeader.module.css";

const SessionHeader: React.FC<React.ComponentProps<typeof Link>> = ({
  children,
  ...rest
}) => {
  return (
    <header className={styles.header}>
      <Link className={styles.backButton} {...rest}>
        <IconChevronLeft />
      </Link>
      <H3>{children}</H3>
    </header>
  );
};

export default SessionHeader;
