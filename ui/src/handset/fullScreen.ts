import { useEffect } from 'react'

/**
 * On the player's own phone, the page full screen, without the browser's bar (owner 2026-11-02: "full screen, as
 * the truck's panel"): a browser goes full screen only for a touch, so at the first one, once, as the phone page
 * does. Not again at every touch while it isn't (owner 2026-11-03: "it goes black, the browser stalls"): while
 * the phone was still going full screen it said it wasn't, and each touch asked again, each time the whole page
 * laid out anew and the browser's notice shown. Left (the phone's back), it's Settings' to ask again.
 */
export function useFullScreen(on: boolean): void {
  useEffect(() => {
    if (!on || !document.fullscreenEnabled) return
    const touched = () => void goFullScreen()
    window.addEventListener('pointerup', touched, { once: true })
    return () => window.removeEventListener('pointerup', touched)
  }, [on])
}

let asking = false

/** The page full screen, unless it is or is being made so. */
export async function goFullScreen(): Promise<void> {
  if (asking || document.fullscreenElement !== null || !document.fullscreenEnabled) return
  asking = true
  try {
    await document.documentElement.requestFullscreen({ navigationUI: 'hide' })
  } catch {
    // The browser said no (a touch it didn't count): the page as it is.
  } finally {
    asking = false
  }
}
