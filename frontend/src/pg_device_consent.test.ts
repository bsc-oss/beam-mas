// @vitest-environment happy-dom

import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

describe("devserver.localice_consent", () => {
  let consentButton: HTMLButtonElement;
  let originalParent: Window;

  beforeEach(() => {
    vi.useFakeTimers();
    vi.resetModules();

    // Reset the DOM
    document.body.innerHTML = "";

    // Create a consent button
    consentButton = document.createElement("button");
    consentButton.name = "action";
    consentButton.value = "consent";
    consentButton.disabled = true;
    consentButton.setAttribute("data-final-text", "Continue");
    consentButton.setAttribute("data-wait-1", "Please wait 1 second");
    consentButton.setAttribute("data-wait-2", "Please wait 2 seconds");
    document.body.appendChild(consentButton);

    // Store original window.parent
    originalParent = window.parent;
  });

  afterEach(() => {
    vi.useRealTimers();

    // Restore window.parent
    Object.defineProperty(window, "parent", {
      value: originalParent,
      writable: true,
    });
  });

  const loadScript = async () => {
    // Dynamically import the module to trigger DOMContentLoaded handler registration
    // @ts-ignore
    await import("./entrypoints/devserver.localice_consent");
    // Dispatch DOMContentLoaded event
    document.dispatchEvent(new Event("DOMContentLoaded"));
  };

  it("should show countdown and enable button after 2 seconds", async () => {
    await loadScript();

    // Initial state - shows 2 seconds remaining
    expect(consentButton.textContent).toBe("Please wait 2 seconds");
    expect(consentButton.disabled).toBe(true);

    // After 1 second - shows 1 second remaining
    vi.advanceTimersByTime(1000);
    expect(consentButton.textContent).toBe("Please wait 1 second");
    expect(consentButton.disabled).toBe(true);

    // After 2 seconds - button is enabled with final text
    vi.advanceTimersByTime(1000);
    expect(consentButton.textContent).toBe("Continue");
    expect(consentButton.disabled).toBe(false);
  });

  it("should use default text when data attributes are missing", async () => {
    consentButton.removeAttribute("data-final-text");
    consentButton.removeAttribute("data-wait-1");
    consentButton.removeAttribute("data-wait-2");

    await loadScript();

    // Initial state - uses default fallback
    expect(consentButton.textContent).toBe("Please wait 2 second(s)");

    vi.advanceTimersByTime(1000);
    expect(consentButton.textContent).toBe("Please wait 1 second(s)");

    vi.advanceTimersByTime(1000);
    expect(consentButton.textContent).toBe("Continue");
    expect(consentButton.disabled).toBe(false);
  });

  it("should do nothing if consent button is not found", async () => {
    document.body.innerHTML = "";

    await loadScript();

    // No errors should be thrown
    vi.advanceTimersByTime(2000);
  });

  it("should do nothing if button is already enabled", async () => {
    consentButton.disabled = false;
    consentButton.textContent = "Already enabled";

    await loadScript();

    // Button text should remain unchanged
    expect(consentButton.textContent).toBe("Already enabled");
    expect(consentButton.disabled).toBe(false);
  });

  it("should do nothing when inside a frame", async () => {
    // Mock being inside a frame
    const mockParent = {} as Window;
    Object.defineProperty(window, "parent", {
      value: mockParent,
      writable: true,
    });

    consentButton.textContent = "Original text";

    await loadScript();

    // Button should remain unchanged when framed
    expect(consentButton.textContent).toBe("Original text");
    expect(consentButton.disabled).toBe(true);
  });

  it("should stop updating after countdown completes", async () => {
    await loadScript();

    // Complete the countdown
    vi.advanceTimersByTime(2000);
    expect(consentButton.textContent).toBe("Continue");
    expect(consentButton.disabled).toBe(false);

    // Advance more time - nothing should change (interval was cleared)
    vi.advanceTimersByTime(5000);
    expect(consentButton.textContent).toBe("Continue");
    expect(consentButton.disabled).toBe(false);
  });
});
