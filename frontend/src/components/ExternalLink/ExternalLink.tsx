// Copyright 2026 Belgian Secure Communications (BSC)
// Copyright 2024, 2025 New Vector Ltd.
// Copyright 2023, 2024 The Matrix.org Foundation C.I.C.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.
//
// Modified by Belgian Secure Communications for Beam application on 2026-04-30

import { Link } from "@pg/compound-web";
import classNames from "classnames";

import styles from "./ExternalLink.module.css";

const ExternalLink: React.FC<React.ComponentProps<typeof Link>> = ({
  children,
  className,
  ...props
}) => (
  <Link
    className={classNames(className, styles.externalLink)}
    target="_blank"
    {...props}
  >
    {children}
  </Link>
);

export default ExternalLink;
