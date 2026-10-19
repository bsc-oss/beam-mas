// Copyright 2026 Element Creations Ltd.
//
// SPDX-License-Identifier: AGPL-3.0-only OR LicenseRef-Element-Commercial
// Please see LICENSE files in the repository root for full details.

// @vitest-environment happy-dom

import { screen } from "@testing-library/react";
import { HttpResponse } from "msw";
import {
  afterAll,
  beforeAll,
  beforeEach,
  describe,
  expect,
  it,
  vi,
} from "vitest";
import { FRAGMENT as COMPAT_SESSION_FRAGMENT } from "../../../src/components/CompatSession"; // PG_CHANGED
import { FRAGMENT as OAUTH2_SESSION_FRAGMENT } from "../../../src/components/OAuth2Session"; // PG_CHANGED
import { FRAGMENT as BROWSER_SESSIONS_FRAGMENT } from "../../../src/components/UserSessionsOverview/BrowserSessionsOverview";
import { makeFragmentData } from "../../../src/gql";
import {
  mockAppSessionsListQuery,
  mockSessionsOverviewQuery,
} from "../../../src/gql/graphql";
import { renderPage, server } from "../render";

describe("Account sessions page", () => {
  // The mocked sessions were last active on 2026-04-23; freeze time shortly
  // after so they don't cross the 90-day inactivity threshold as time passes
  beforeAll(() => {
    vi.useFakeTimers({ toFake: ["Date"] });
    vi.setSystemTime(new Date("2026-05-01T00:00:00Z"));
  });

  afterAll(() => vi.useRealTimers());

  // PG_CHANGED - the current device is remembered for the visit, not for one test
  beforeEach(() => sessionStorage.clear());

  it("renders the page", async () => {
    const { asFragment } = await renderPage("/sessions");
    expect(asFragment()).toMatchSnapshot();
  });

  describe("session limit", () => {
    it("displays an error if they've hit the session soft_limit", async () => {
      server.use(
        mockSessionsOverviewQuery(() =>
          HttpResponse.json({
            data: {
              viewer: {
                __typename: "User",
                id: "123",
                ...makeFragmentData(
                  {
                    browserSessions: {
                      totalCount: 3,
                    },
                  },
                  BROWSER_SESSIONS_FRAGMENT,
                ),
                unfilteredAppSessions: {
                  // They have 12 sessions
                  totalCount: 12,
                },
              },
              siteConfig: {
                sessionLimit: {
                  // The limit is 12
                  softLimit: 12,
                },
              },
            },
          }),
        ),
      );

      server.use(
        mockAppSessionsListQuery(() =>
          HttpResponse.json({
            data: {
              viewer: {
                __typename: "User",
                id: "123",
                appSessions: {
                  totalCount: 12,
                  // Edge for each device on the page (6 per page):
                  // { cursor: "", node: { __typename: "Oauth2Session", ... } }
                  // { cursor: "", node: { __typename: "CompatSession",... } }
                  edges: Array.from(Array(6).keys()).map((index) => ({
                    cursor: `cursor${index}`,
                    node: {
                      __typename: "CompatSession",
                      id: `compat_session:${index}`,
                      createdAt: "2026-04-23T21:25:43.353610+00:00",
                      deviceId: `zzzZZZzzz${index}`,
                      finishedAt: null,
                      lastActiveIp: "127.0.0.1",
                      lastActiveAt: "2026-04-23T21:25:43.367193+00:00",
                      humanName: "Jungle Phone",
                      userAgent: {
                        name: "Chrome",
                        os: "Linux",
                        model: null,
                        deviceType: "PC",
                      },
                      ssoLogin: null,
                    },
                  })),
                  // Doesn't matter for test
                  pageInfo: {
                    startCursor: "foo",
                    endCursor: "bar",
                    hasNextPage: false,
                    hasPreviousPage: true,
                  },
                },
              },
            },
          }),
        ),
      );

      const { asFragment } = await renderPage("/sessions");

      // Make sure there is an error on the page
      //
      // FIXME: Ideally, we'd use a more accessible way to look for the error on the
      // page (like `getByRole(...)`) but there doesn't seem to be an appropriate
      // `aria-role` for this kind of thing. `aria-role="status"` is for the result of
      // the user taking some action whereas this is just an error that's displayed
      // informationally on page load.
      //
      // Perhaps the best we could do is add a generic `aria-role="region"` with
      // `aria-labelledby="error-heading"` so we could use `getByRole('region', { name:
      // "You've hit the device limit"})` but this would require some changes to the
      // `Alert` component in Compound itself.
      screen.getByTestId("device-limit-error");

      // Sanity check page overall
      expect(asFragment()).toMatchSnapshot();
    });
  });

  // PG_CHANGED
  describe("current device", () => {
    const mockOneCompatSession = (deviceId: string): void => {
      server.use(
        mockSessionsOverviewQuery(() =>
          HttpResponse.json({
            data: {
              viewer: {
                __typename: "User",
                id: "123",
                ...makeFragmentData(
                  { browserSessions: { totalCount: 1 } },
                  BROWSER_SESSIONS_FRAGMENT,
                ),
                unfilteredAppSessions: { totalCount: 1 },
              },
              siteConfig: { sessionLimit: null },
            },
          }),
        ),
        mockAppSessionsListQuery(() =>
          HttpResponse.json({
            data: {
              viewer: {
                __typename: "User",
                id: "123",
                appSessions: {
                  totalCount: 1,
                  edges: [
                    {
                      cursor: "cursor0",
                      node: {
                        __typename: "CompatSession" as const,
                        ...makeFragmentData(
                          {
                            id: "compat_session:0",
                            createdAt: "2026-04-23T21:25:43.353610+00:00",
                            deviceId,
                            finishedAt: null,
                            lastActiveIp: "127.0.0.1",
                            lastActiveAt: "2026-04-23T21:25:43.367193+00:00",
                            humanName: "Jungle Phone",
                            userAgent: {
                              name: "Chrome",
                              os: "Linux",
                              model: null,
                              deviceType: "PC",
                            },
                            ssoLogin: null,
                          },
                          COMPAT_SESSION_FRAGMENT,
                        ),
                      },
                    },
                  ],
                  pageInfo: {
                    startCursor: "foo",
                    endCursor: "bar",
                    hasNextPage: false,
                    hasPreviousPage: false,
                  },
                },
              },
            },
          }),
        ),
      );
    };

    // Drives the whole path the Beam apps take: the account action URL, the redirect
    // onto the device list, and the hand-off through session storage in between.
    it("marks the device the apps said they are", async () => {
      mockOneCompatSession("zzzZZZzzz0");

      await renderPage(
        "/?action=org.matrix.devices_list&pg_current_device_id=zzzZZZzzz0",
      );

      expect(screen.queryByText("Current")).not.toBeNull();
    });

    // The "manage account and devices" entry in the apps links here with no action at
    // all, landing on the account home rather than the device list. The device id has to
    // survive that, and the walk to the device list afterwards.
    it("marks the device when the apps linked here without an action", async () => {
      mockOneCompatSession("zzzZZZzzz0");

      const { unmount } = await renderPage("/?pg_current_device_id=zzzZZZzzz0");
      unmount();
      await renderPage("/sessions");

      expect(screen.queryByText("Current")).not.toBeNull();
    });

    // The real Beam sessions are OAuth2 ones, whose device id has to be dug out of the
    // scope. Beam grants a logout scope after the device scope, so this is the shape that
    // actually reaches the page.
    it("marks an oauth2 session whose scope carries more than the device", async () => {
      server.use(
        mockSessionsOverviewQuery(() =>
          HttpResponse.json({
            data: {
              viewer: {
                __typename: "User",
                id: "123",
                ...makeFragmentData(
                  { browserSessions: { totalCount: 1 } },
                  BROWSER_SESSIONS_FRAGMENT,
                ),
                unfilteredAppSessions: { totalCount: 1 },
              },
              siteConfig: { sessionLimit: null },
            },
          }),
        ),
        mockAppSessionsListQuery(() =>
          HttpResponse.json({
            data: {
              viewer: {
                __typename: "User",
                id: "123",
                appSessions: {
                  totalCount: 1,
                  edges: [
                    {
                      cursor: "cursor0",
                      node: {
                        __typename: "Oauth2Session" as const,
                        ...makeFragmentData(
                          {
                            id: "oauth2_session:0",
                            scope:
                              "openid urn:matrix:org.matrix.msc2967.client:api:* urn:matrix:org.matrix.msc2967.client:device:ii1XeEfJb0 urn:pg:mas-logout",
                            createdAt: "2026-04-23T21:25:43.353610+00:00",
                            finishedAt: null,
                            lastActiveIp: "127.0.0.1",
                            lastActiveAt: "2026-04-23T21:25:43.367193+00:00",
                            humanName: "Google Pixel 8a",
                            userAgent: null,
                            client: {
                              id: "client:0",
                              clientId: "beam",
                              clientName: "Beam",
                              clientUri: "https://beam.example",
                              logoUri: null,
                              applicationType: "NATIVE" as const,
                            },
                          },
                          OAUTH2_SESSION_FRAGMENT,
                        ),
                      },
                    },
                  ],
                  pageInfo: {
                    startCursor: "foo",
                    endCursor: "bar",
                    hasNextPage: false,
                    hasPreviousPage: false,
                  },
                },
              },
            },
          }),
        ),
      );

      await renderPage(
        "/?action=org.matrix.devices_list&pg_current_device_id=ii1XeEfJb0",
      );

      expect(screen.queryByText("Current")).not.toBeNull();
    });

    it("marks nothing when the apps did not say", async () => {
      mockOneCompatSession("zzzZZZzzz0");

      await renderPage("/?action=org.matrix.devices_list");

      expect(screen.queryByText("Current")).toBeNull();
    });

    it("marks nothing when the device is no longer in the list", async () => {
      mockOneCompatSession("some-other-device");

      await renderPage(
        "/?action=org.matrix.devices_list&pg_current_device_id=zzzZZZzzz0",
      );

      expect(screen.queryByText("Current")).toBeNull();
    });
  });
});
