// PG_CHANGED - disables the continue button by default on device consent page

document.addEventListener("DOMContentLoaded", () => {
  const consentButton = document.querySelector<HTMLButtonElement>(
    'button[name="action"][value="consent"]',
  );
  if (consentButton == null || consentButton.disabled === false) return;

  // Safe framing check that doesn't cross origins
  const isFramed = window.parent !== window; // true if inside a frame
  if (isFramed) return;

  const finalText = consentButton.getAttribute("data-final-text") ?? "Continue";
  const waitText = (n: number) =>
    consentButton.getAttribute(`data-wait-${n}`) ??
    `Please wait ${n} second(s)`;

  let remaining = 2;
  consentButton.textContent = waitText(remaining);

  const timer = setInterval(() => {
    remaining -= 1;
    if (remaining > 0) {
      consentButton.textContent = waitText(remaining);
    } else {
      clearInterval(timer);
      consentButton.disabled = false;
      consentButton.textContent = finalText;
    }
  }, 1000);
});
