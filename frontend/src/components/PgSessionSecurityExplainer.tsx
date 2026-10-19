// Copyright 2026 Belgian Secure Communications (BSC)
//
// SPDX-License-Identifier: AGPL-3.0-only
// Please see LICENSE files in the repository root for full details.

import IconInfo from "@vector-im/compound-design-tokens/assets/web/icons/info";
import { Text } from "@vector-im/compound-web";
import { Trans, useTranslation } from "react-i18next";
import * as Collapsible from "./Collapsible";
import styles from "./PgSessionSecurityExplainer.module.css";

const Paragraph: React.FC<React.PropsWithChildren> = ({ children }) => (
  <Text type="body" weight="regular" size="md" className="text-secondary">
    {children}
  </Text>
);

/**
 * Explains what an unverified device is and what to do about it, why stale or
 * unrecognised sessions should be removed, how to spot them, and how devices differ
 * from browsers.
 *
 * The Beam apps link straight to these pages when the account has unverified devices,
 * so most people reading this have been sent here by a warning and have never seen the
 * page before. That is why the unverified devices section comes first.
 */
const PgSessionSecurityExplainer: React.FC = () => {
  const { t } = useTranslation();

  const causes = [
    t("frontend.pg_session_security.unverified_cause_identity_reset"),
    t("frontend.pg_session_security.unverified_cause_web_login"),
    t("frontend.pg_session_security.unverified_cause_skipped_verification"),
  ];

  const sections = [
    {
      title: t("frontend.pg_session_security.unverified_heading"),
      content: (
        <div className="flex flex-col gap-4">
          <Paragraph>
            <Trans
              i18nKey="frontend.pg_session_security.unverified_body"
              components={{ strong: <strong /> }}
            />
          </Paragraph>
          <div className="flex flex-col gap-2">
            <Paragraph>
              {t("frontend.pg_session_security.unverified_causes_intro")}
            </Paragraph>
            <ul className="list-disc list-outside pl-6">
              {causes.map((cause) => (
                <Text
                  as="li"
                  key={cause}
                  type="body"
                  weight="regular"
                  size="md"
                  className="text-secondary"
                >
                  {cause}
                </Text>
              ))}
            </ul>
          </div>
          <Paragraph>
            <Trans
              i18nKey="frontend.pg_session_security.unverified_compromise_note"
              components={{ strong: <strong /> }}
            />
          </Paragraph>
        </div>
      ),
    },
    {
      title: t("frontend.pg_session_security.why_heading"),
      content: (
        <Paragraph>
          <Trans
            i18nKey="frontend.pg_session_security.why_body"
            components={{ strong: <strong /> }}
          />
        </Paragraph>
      ),
    },
    {
      title: t("frontend.pg_session_security.spot_heading"),
      content: (
        <Paragraph>
          {/* The labels are interpolated so they always match the ones rendered on the
              session cards, in whichever language the page is displayed. */}
          <Trans
            i18nKey="frontend.pg_session_security.spot_body"
            components={{ strong: <strong /> }}
            values={{
              lastActiveLabel: t("frontend.session.last_active_label"),
              signedInLabel: t("frontend.session.signed_in_label"),
            }}
          />
        </Paragraph>
      ),
    },
    {
      title: t("frontend.pg_session_security.devices_vs_browsers_heading"),
      content: (
        <Paragraph>
          <Trans
            i18nKey="frontend.pg_session_security.devices_vs_browsers_body"
            components={{ strong: <strong /> }}
          />
        </Paragraph>
      ),
    },
  ];

  return (
    <div className={styles.card}>
      <div className={styles.header}>
        <IconInfo aria-hidden />
        <Text type="body" weight="semibold" size="sm">
          {t("frontend.pg_session_security.title")}
        </Text>
      </div>

      {sections.map(({ title, content }) => (
        <Collapsible.Section key={title} title={title}>
          {content}
        </Collapsible.Section>
      ))}
    </div>
  );
};

export default PgSessionSecurityExplainer;
