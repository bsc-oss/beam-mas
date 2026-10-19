// Copyright 2024, 2025 New Vector Ltd.
// Copyright 2023, 2024 The Matrix.org Foundation C.I.C.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

import type { UseMutationResult } from "@tanstack/react-query";
import IconDelete from "@vector-im/compound-design-tokens/assets/web/icons/delete";
import { Alert, Button } from "@vector-im/compound-web"; // PG_CHANGED
import { useState } from "react";
import { useTranslation } from "react-i18next";
import * as Dialog from "../Dialog";
import LoadingSpinner from "../LoadingSpinner/LoadingSpinner";

/**
 * Generic end session button
 * Handles loading state while endSession is in progress
 */
const EndSessionButton: React.FC<
  React.PropsWithChildren<{
    mutation: UseMutationResult<unknown, unknown, void>;
    size: "md" | "lg";
    // PG_CHANGED - set when this session is the one the page is being used from. Which kind it
    // is changes what actually happens: ending the browser session signs you out of this page.
    sessionKind?: "device" | "browser";
  }>
> = ({ children, mutation, size, sessionKind }) => {
  const [open, setOpen] = useState(false);
  const { t } = useTranslation();

  // PG_CHANGED - warning shown when the session being ended is the one in use
  const CURRENT_WARNINGS = {
    device: {
      title: "frontend.pg_end_current_device_warning.title",
      body: "frontend.pg_end_current_device_warning.body",
    },
    browser: {
      title: "frontend.pg_end_current_browser_warning.title",
      body: "frontend.pg_end_current_browser_warning.body",
    },
  } as const;

  const currentWarning = sessionKind ? CURRENT_WARNINGS[sessionKind] : null;

  const onConfirm = (e: React.MouseEvent<HTMLButtonElement>): void => {
    e.preventDefault();
    mutation.mutate(void 0, { onSuccess: () => setOpen(false) });
  };

  return (
    <Dialog.Dialog
      open={open}
      onOpenChange={setOpen}
      trigger={
        <Button kind="secondary" destructive size={size} Icon={IconDelete}>
          {t("frontend.end_session_button.text")}
        </Button>
      }
    >
      <Dialog.Title>
        {t("frontend.end_session_button.confirmation_modal_title")}
      </Dialog.Title>

      <Dialog.Description>
        {t("frontend.end_session_button.confirmation_modal_body_text")}
      </Dialog.Description>

      {/* PG_CHANGED - signing this one out is almost never what was meant */}
      {currentWarning && (
        <Alert type="critical" title={t(currentWarning.title)}>
          {t(currentWarning.body)}
        </Alert>
      )}

      {children && <Dialog.Description asChild>{children}</Dialog.Description>}

      <Button
        type="button"
        kind="primary"
        destructive
        onClick={onConfirm}
        disabled={mutation.isPending}
        Icon={mutation.isPending ? undefined : IconDelete}
      >
        {mutation.isPending && <LoadingSpinner inline />}
        {t("frontend.end_session_button.text")}
      </Button>

      <Dialog.Close asChild>
        <Button kind="tertiary">{t("action.cancel")}</Button>
      </Dialog.Close>
    </Dialog.Dialog>
  );
};

export default EndSessionButton;
