// Copyright 2024, 2025 New Vector Ltd.
// Copyright 2023, 2024 The Matrix.org Foundation C.I.C.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

// @vitest-environment happy-dom

import userEvent from "@testing-library/user-event";
import { TooltipProvider } from "@vector-im/compound-web";
import { beforeAll, describe, expect, it } from "vitest";
import { makeFragmentData } from "../gql";
import { mockLocale } from "../test-utils/mockLocale";
import render from "../test-utils/render";
import CompatSession, { FRAGMENT } from "./CompatSession";

describe("<CompatSession />", () => {
  const baseSession = {
    id: "session-id",
    deviceId: "abcd1234",
    createdAt: "2023-06-29T03:35:17.451292+00:00",
    finishedAt: null,
    lastActiveIp: "1.2.3.4",
    lastActiveAt: null,
    humanName: null,
    userAgent: null,
    ssoLogin: {
      id: "test-id",
      redirectUri: "https://element.io/",
    },
  };

  const finishedAt = "2023-06-29T03:35:19.451292+00:00";

  beforeAll(() => mockLocale());

  it("renders an active session", () => {
    const session = makeFragmentData(baseSession, FRAGMENT);
    const { asFragment } = render(<CompatSession session={session} />);
    expect(asFragment()).toMatchSnapshot();
  });

  // PG_CHANGED
  it("marks the device the page was opened from", () => {
    const session = makeFragmentData(baseSession, FRAGMENT);
    const { queryByText } = render(
      <CompatSession session={session} currentDeviceId="abcd1234" />,
    );
    expect(queryByText("Current")).not.toBeNull();
  });

  // PG_CHANGED
  it("does not mark any device when the page was not opened from one", () => {
    const session = makeFragmentData(baseSession, FRAGMENT);
    const { queryByText } = render(<CompatSession session={session} />);
    expect(queryByText("Current")).toBeNull();
  });

  // PG_CHANGED - deviceId is nullable, and the sessions list passes null rather than
  // undefined when the page was not opened from an app, so both sides can be null at once
  it("does not mark a session that has no device id", () => {
    const session = makeFragmentData(
      { ...baseSession, deviceId: null },
      FRAGMENT,
    );
    const { queryByText } = render(
      <CompatSession session={session} currentDeviceId={null} />,
    );
    expect(queryByText("Current")).toBeNull();
  });

  // PG_CHANGED
  it("warns before signing out the device the page was opened from", async () => {
    const user = userEvent.setup();
    const session = makeFragmentData(baseSession, FRAGMENT);
    const { getByRole, queryByText } = render(
      <TooltipProvider>
        <CompatSession session={session} currentDeviceId="abcd1234" />
      </TooltipProvider>,
    );

    await user.click(getByRole("button", { name: "Remove device" }));

    expect(queryByText("This is the device you are using")).not.toBeNull();
  });

  // PG_CHANGED
  it("does not warn before signing out any other device", async () => {
    const user = userEvent.setup();
    const session = makeFragmentData(baseSession, FRAGMENT);
    const { getByRole, queryByText } = render(
      <TooltipProvider>
        <CompatSession session={session} />
      </TooltipProvider>,
    );

    await user.click(getByRole("button", { name: "Remove device" }));

    expect(queryByText("This is the device you are using")).toBeNull();
  });

  it("renders a finished session", () => {
    const session = makeFragmentData(
      {
        ...baseSession,
        finishedAt,
      },
      FRAGMENT,
    );
    const { asFragment } = render(<CompatSession session={session} />);
    expect(asFragment()).toMatchSnapshot();
  });
});
