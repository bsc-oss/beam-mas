//
// Copyright 2026 Belgian Secure Communications (BSC)
//
// SPDX-License-Identifier: AGPL-3.0-only
// Please see LICENSE files in the repository root for full details.
//

// PG_CHANGED - prevents double form submissions. A second POST is at best a
// no-op and at worst an error for the user, as the handler has already consumed
// the single-use session cookie (e.g. "Missing session cookie" on the upstream
// OAuth 2.0 registration page).
//
// Cancelling the second submit event is what stops the second POST; the buttons
// only get `aria-disabled` to show that. We deliberately don't set `disabled`:
// disabled controls are left out of the form data, which would drop the
// name/value of the very button that submitted the form.

const submitButtons = (form: HTMLFormElement): NodeListOf<HTMLButtonElement> =>
  form.querySelectorAll<HTMLButtonElement>(
    "button[type=submit], button:not([type])",
  );

const guardedForms = (): NodeListOf<HTMLFormElement> =>
  document.querySelectorAll<HTMLFormElement>("form.cpd-form-root");

document.addEventListener("DOMContentLoaded", () => {
  for (const form of guardedForms()) {
    // Only ever attach once per form, in case this script is loaded twice on a
    // page
    if (form.dataset.pgGuarded === "true") continue;
    form.dataset.pgGuarded = "true";

    form.addEventListener("submit", (event) => {
      if (form.dataset.pgSubmitting === "true") {
        event.preventDefault();
        return;
      }
      form.dataset.pgSubmitting = "true";

      for (const button of submitButtons(form)) {
        button.setAttribute("aria-disabled", "true");
      }
    });
  }
});

// Coming back to the page through the back/forward cache restores the DOM as we
// left it, so unlock the forms we locked to keep them usable
window.addEventListener("pageshow", (event) => {
  if (!event.persisted) return;

  for (const form of guardedForms()) {
    if (form.dataset.pgSubmitting !== "true") continue;
    delete form.dataset.pgSubmitting;

    for (const button of submitButtons(form)) {
      button.removeAttribute("aria-disabled");
    }
  }
});
