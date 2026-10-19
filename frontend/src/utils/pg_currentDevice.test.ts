// Copyright 2026 Belgian Secure Communications (BSC)
//
// SPDX-License-Identifier: AGPL-3.0-only
// Please see LICENSE files in the repository root for full details.

// @vitest-environment happy-dom

import { beforeEach, describe, expect, it } from "vitest";
import {
  getCurrentDeviceId,
  rememberCurrentDeviceId,
} from "./pg_currentDevice";

describe("pg_currentDevice", () => {
  beforeEach(() => {
    sessionStorage.clear();
  });

  it("returns null when nothing was remembered", () => {
    expect(getCurrentDeviceId()).toBeNull();
  });

  it("keeps the device id for the rest of the visit", () => {
    rememberCurrentDeviceId("ABCD1234");
    expect(getCurrentDeviceId()).toBe("ABCD1234");
  });

  it("keeps the most recently remembered device id", () => {
    rememberCurrentDeviceId("ABCD1234");
    rememberCurrentDeviceId("EFGH5678");
    expect(getCurrentDeviceId()).toBe("EFGH5678");
  });
});
