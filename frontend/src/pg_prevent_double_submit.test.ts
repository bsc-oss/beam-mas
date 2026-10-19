//
// Copyright 2026 Belgian Secure Communications (BSC)
//
// SPDX-License-Identifier: AGPL-3.0-only
// Please see LICENSE files in the repository root for full details.
//

// @vitest-environment happy-dom

import { beforeEach, describe, expect, it, vi } from "vitest";

describe("pg_prevent_double_submit", () => {
  let form: HTMLFormElement;
  let submitButton: HTMLButtonElement;

  beforeEach(() => {
    vi.resetModules();

    // A form shaped like the device consent one, where the submit button itself
    // carries the action the handler needs
    document.body.innerHTML = `
      <form method="POST" class="cpd-form-root">
        <input type="hidden" name="csrf" value="token" />
        <button type="submit" name="action" value="consent">Continue</button>
      </form>
    `;

    form = document.querySelector("form") as HTMLFormElement;
    submitButton = form.querySelector("button") as HTMLButtonElement;
  });

  const loadScript = async () => {
    // @ts-expect-error the entrypoint has no exports
    await import("./entrypoints/pg_prevent_double_submit");
    document.dispatchEvent(new Event("DOMContentLoaded"));
  };

  /** Submit the form, returning whether the submission went through */
  const submit = (): boolean =>
    form.dispatchEvent(
      new Event("submit", { bubbles: true, cancelable: true }),
    );

  /**
   * happy-dom's PageTransitionEvent drops the `persisted` flag, so set it
   * ourselves
   */
  const pageShow = (persisted: boolean): void => {
    const event = new Event("pageshow");
    Object.defineProperty(event, "persisted", { value: persisted });
    window.dispatchEvent(event);
  };

  it("locks the submit button once the form is submitted", async () => {
    await loadScript();

    expect(submit()).toBe(true);
    expect(submitButton.getAttribute("aria-disabled")).toBe("true");
  });

  it("cancels a second submission", async () => {
    await loadScript();

    expect(submit()).toBe(true);
    expect(submit()).toBe(false);
  });

  it("never marks the button as disabled", async () => {
    await loadScript();

    submit();

    // A disabled control is left out of the form data, which would drop the
    // submitter's name/value - here, the `action` the handler requires. Note
    // that happy-dom does not model that exclusion, so this asserts the
    // contract rather than the browser behaviour it protects against
    expect(submitButton.disabled).toBe(false);
    expect(submitButton.hasAttribute("disabled")).toBe(false);
  });

  it("unlocks the form when restored from the back/forward cache", async () => {
    await loadScript();

    submit();
    expect(submitButton.getAttribute("aria-disabled")).toBe("true");

    pageShow(true);

    expect(submitButton.hasAttribute("aria-disabled")).toBe(false);
    // ...and the form can be submitted again
    expect(submit()).toBe(true);
  });

  it("stays locked on a normal page load", async () => {
    await loadScript();

    submit();
    pageShow(false);

    expect(submitButton.getAttribute("aria-disabled")).toBe("true");
  });

  it("leaves untouched forms alone when restored from the cache", async () => {
    await loadScript();

    // Never submitted, so there is nothing to unlock
    pageShow(true);

    expect(form.dataset.pgSubmitting).toBeUndefined();
    expect(submit()).toBe(true);
  });

  it("does nothing when there is no form on the page", async () => {
    document.body.innerHTML = "";

    await loadScript();
  });
});
