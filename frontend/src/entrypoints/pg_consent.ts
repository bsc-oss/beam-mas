//
// Copyright 2026 Belgian Secure Communications (BSC)
//
// SPDX-License-Identifier: AGPL-3.0-only
// Please see LICENSE files in the repository root for full details.
//

// Prevent double form submission on the consent page
document.addEventListener("DOMContentLoaded", () => {
  const form = document.querySelector<HTMLFormElement>("form.cpd-form-root");
  if (!form) return;

  form.addEventListener("submit", () => {
    const btn = form.querySelector<HTMLButtonElement>("button[type=submit]");
    if (btn) btn.disabled = true;
  });
});
