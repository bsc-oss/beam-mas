// Copyright 2026 Belgian Secure Communications (BSC)
//
// SPDX-License-Identifier: AGPL-3.0-only
// Please see LICENSE files in the repository root for full details.

// @vitest-environment happy-dom

import userEvent from "@testing-library/user-event";
import { TooltipProvider } from "@vector-im/compound-web";
import { beforeAll, describe, expect, it } from "vitest";
import { mockLocale } from "../test-utils/mockLocale";
import render from "../test-utils/render";
import PgSessionSecurityExplainer from "./PgSessionSecurityExplainer";

describe("<PgSessionSecurityExplainer />", () => {
  beforeAll(() => mockLocale());

  // The bodies carry <strong> markers and one carries placeholders, both of which go through
  // Trans rather than a plain t() call. A mistake in either renders the markup literally.
  it("fills in the session card labels and leaves no markup on screen", async () => {
    const user = userEvent.setup();
    const { container, getByText } = render(
      <TooltipProvider>
        <PgSessionSecurityExplainer />
      </TooltipProvider>,
    );

    await user.click(getByText("How to spot one"));

    // The labels come from the session cards, so they must be interpolated, not printed raw.
    expect(container.textContent).toContain("Last Active");
    expect(container.textContent).toContain("Signed in");
    expect(container.textContent).not.toContain("{{");
    expect(container.textContent).not.toContain("<strong>");
  });

  it("emphasises the phrase that changes what the reader does", async () => {
    const user = userEvent.setup();
    const { container, getByText } = render(
      <TooltipProvider>
        <PgSessionSecurityExplainer />
      </TooltipProvider>,
    );

    await user.click(getByText("Unverified devices"));

    const emphasised = Array.from(container.querySelectorAll("strong")).map(
      (element) => element.textContent,
    );
    expect(emphasised).toContain("this can only be done on the device itself");
  });
});
