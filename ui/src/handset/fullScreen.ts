/**
 * On the player's own phone, the page full screen, without the browser's bar (owner 2026-11-02: "full screen, as
 * the truck's panel"), from Settings only: going full screen at the first touch, the page stalled as it did
 * (the phone's report: its frames and its touches lost going full screen and back), and the touch that did it
 * went nowhere (owner 2026-11-03: "take out going full screen at a touch").
 */

let asking = false

/** The page full screen, unless it is or is being made so. */
export async function goFullScreen(): Promise<void> {
  if (asking || document.fullscreenElement !== null || !document.fullscreenEnabled) return
  asking = true
  try {
    await document.documentElement.requestFullscreen({ navigationUI: 'hide' })
  } catch {
    // The browser said no: the page as it is.
  } finally {
    asking = false
  }
}
