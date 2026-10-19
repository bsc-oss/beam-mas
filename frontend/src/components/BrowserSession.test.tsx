// Copyright 2026 Belgian Secure Communications (BSC)
//
// SPDX-License-Identifier: AGPL-3.0-only
// Please see LICENSE files in the repository root for full details.

// @vitest-environment happy-dom

import userEvent from "@testing-library/user-event";
import { TooltipProvider } from "@vector-im/compound-web";
import { beforeAll, describe, expect, it } from "vitest";
import { makeFragmentData } from "../gql";
import type { DeviceType } from "../gql/graphql";
import { mockLocale } from "../test-utils/mockLocale";
import render from "../test-utils/render";
import BrowserSession, { FRAGMENT } from "./BrowserSession";

describe("<BrowserSession />", () => {
  const baseSession = {
    id: "browser_session:0",
    createdAt: "2026-04-23T21:25:43.353610+00:00",
    finishedAt: null,
    lastActiveAt: "2026-04-23T21:25:43.367193+00:00",
    userAgent: {
      name: "Chrome",
      os: "Linux",
      model: null,
      deviceType: "PC" as DeviceType,
    },
  };

  beforeAll(() => mockLocale());

  // Ending this one signs you out of the page you are reading, so it warrants its own warning
  // rather than the one written for devices.
  it("warns before signing out the browser the page is being used from", async () => {
    const user = userEvent.setup();
    const session = makeFragmentData(baseSession, FRAGMENT);
    const { getByRole, queryByText } = render(
      <TooltipProvider>
        <BrowserSession session={session} isCurrent={true} />
      </TooltipProvider>,
    );

    await user.click(getByRole("button", { name: "Remove device" }));

    expect(queryByText("This is the browser you are using")).not.toBeNull();
    expect(queryByText("This is the device you are using")).toBeNull();
  });

  it("does not warn before signing out any other browser", async () => {
    const user = userEvent.setup();
    const session = makeFragmentData(baseSession, FRAGMENT);
    const { getByRole, queryByText } = render(
      <TooltipProvider>
        <BrowserSession session={session} isCurrent={false} />
      </TooltipProvider>,
    );

    await user.click(getByRole("button", { name: "Remove device" }));

    expect(queryByText("This is the browser you are using")).toBeNull();
  });
});
