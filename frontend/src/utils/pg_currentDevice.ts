// Copyright 2026 Belgian Secure Communications (BSC)
//
// SPDX-License-Identifier: AGPL-3.0-only
// Please see LICENSE files in the repository root for full details.

const STORAGE_KEY = "pg.currentDeviceId";

/**
 * The Beam apps append their own device id to the account management URL, so the device
 * list can point out which entry is the device in your hand.
 *
 * It only ever arrives on the very first load: the account route redirects to the
 * device list with a clean search, and its schema drops keys it does not know about. So
 * it is kept for the rest of the visit, and deliberately not for longer than that — it
 * describes how this page was opened, not the account.
 */
export const rememberCurrentDeviceId = (deviceId: string): void => {
  try {
    sessionStorage.setItem(STORAGE_KEY, deviceId);
  } catch {
    // Storage can be refused, for instance in private browsing. Pointing out the
    // current device is a nicety and is not worth failing the page over.
  }
};

export const getCurrentDeviceId = (): string | null => {
  try {
    return sessionStorage.getItem(STORAGE_KEY);
  } catch {
    return null;
  }
};
