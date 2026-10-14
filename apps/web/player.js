/* The native audio element owns decoding, buffering and platform media controls. */
(() => {
  const audio = () => document.getElementById("vela-audio");
  document.addEventListener("keydown", (event) => {
    if (document.querySelector('[role="dialog"]') || event.altKey || event.ctrlKey || event.metaKey || /INPUT|TEXTAREA|SELECT|BUTTON|A/.test(event.target.tagName) || event.target.isContentEditable) return;
    if (event.code === "Space") { const button = document.getElementById("player-toggle"); if (button) { event.preventDefault(); button.click(); } }
    if (event.code === "ArrowRight" && audio()) { event.preventDefault(); audio().currentTime = Math.min(audio().duration || 0, audio().currentTime + 10); }
    if (event.code === "ArrowLeft" && audio()) { event.preventDefault(); audio().currentTime = Math.max(0, audio().currentTime - 10); }
  });
  if ("mediaSession" in navigator) {
    const button = (id) => () => document.getElementById(id)?.click();
    navigator.mediaSession.setActionHandler("play", () => { if (audio()?.paused) button("player-toggle")(); });
    navigator.mediaSession.setActionHandler("pause", () => { if (audio() && !audio().paused) button("player-toggle")(); });
    navigator.mediaSession.setActionHandler("previoustrack", button("player-previous"));
    navigator.mediaSession.setActionHandler("nexttrack", button("player-next"));
    navigator.mediaSession.setActionHandler("seekto", ({ seekTime }) => { if (audio() && Number.isFinite(seekTime)) audio().currentTime = seekTime; });
  }
  document.addEventListener("vela-track", ({ detail }) => {
    if ("mediaSession" in navigator && "MediaMetadata" in window) navigator.mediaSession.metadata = new MediaMetadata(detail);
  });
  document.addEventListener("play", (event) => { if (event.target.id === "vela-audio" && "mediaSession" in navigator) navigator.mediaSession.playbackState = "playing"; }, true);
  document.addEventListener("pause", (event) => { if (event.target.id === "vela-audio" && "mediaSession" in navigator) navigator.mediaSession.playbackState = "paused"; }, true);
  const focusable = (dialog) => [...dialog.querySelectorAll('button:not(:disabled), input:not(:disabled), select:not(:disabled), a[href], [tabindex="0"]')];
  let activeDialog = null;
  let returnFocus = null;
  new MutationObserver(() => {
    const dialog = document.querySelector('[role="dialog"]');
    if (dialog && dialog !== activeDialog) {
      returnFocus = document.activeElement;
      activeDialog = dialog;
      focusable(dialog)[0]?.focus();
    } else if (!dialog && activeDialog) {
      activeDialog = null;
      if (returnFocus?.isConnected) returnFocus.focus();
    }
  }).observe(document.getElementById("main"), { childList: true, subtree: true });
  document.addEventListener("keydown", (event) => {
    if (!activeDialog) return;
    if (event.key === "Escape") activeDialog.querySelector('button[aria-label="Close upload dialog"]:not(:disabled)')?.click();
    if (event.key !== "Tab") return;
    const controls = focusable(activeDialog);
    const first = controls[0];
    const last = controls.at(-1);
    if (event.shiftKey && (document.activeElement === first || !activeDialog.contains(document.activeElement))) { event.preventDefault(); last?.focus(); }
    else if (!event.shiftKey && (document.activeElement === last || !activeDialog.contains(document.activeElement))) { event.preventDefault(); first?.focus(); }
  });
})();
