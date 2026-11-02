import { useEffect } from 'react'

/**
 * On the player's own phone, the page full screen, without the browser's bar (owner 2026-11-02: "full screen,
 * as the truck's panel"): a browser goes full screen only for a touch, so at each touch while it isn't (the first,
 * and again after the phone's back button left it).
 */
export function useFullScreen(on: boolean): void {
  useEffect(() => {
    if (!on || !document.fullscreenEnabled) return
    const touched = () => {
      if (document.fullscreenElement === null) {
        void document.documentElement.requestFullscreen({ navigationUI: 'hide' }).catch(() => {})
      }
    }
    window.addEventListener('pointerup', touched)
    return () => window.removeEventListener('pointerup', touched)
  }, [on])
}
