import { useEffect, useRef, useState } from 'react'
import { stepped } from '../cabin/apps/keybinds'
import { stopFade } from '../cabin/apps/fade'
import { useNowPlaying } from '../cabin/dashboard/useNowPlaying'
import { postToHost } from '../shared/host'
import { PLAYERS } from '../phone/PhoneVolume'

/** How long the volume stays shown after the last press (ms). */
const SHOWN_MS = 1400

/**
 * The phone's volume keys on its left side, as an iPhone's (owner 2026-11-21: "two little buttons on the side of
 * the phone, like a real phone's volume up"): with TikTok open, TikTok's volume (its page shows it); else the
 * volume of what plays (Now Playing's player, as the cab's keybinds set it), shown on the phone as iOS shows it.
 */
export function SideKeys({ tiktok }: { tiktok: boolean }) {
  const { showing } = useNowPlaying()
  const player = showing ? PLAYERS[showing] : null
  const [shown, setShown] = useState<number | null>(null)
  const hiding = useRef<number | undefined>(undefined)
  useEffect(() => () => window.clearTimeout(hiding.current), [])

  const press = (direction: 1 | -1) => {
    if (tiktok) {
      postToHost({ type: 'tiktok', command: { action: 'volume', step: direction } })
      return
    }
    const volume = player?.volume() ?? null
    if (!player || volume === null) return
    stopFade(player.name)
    const next = stepped(volume, direction)
    player.setVolume(next)
    setShown(next)
    window.clearTimeout(hiding.current)
    hiding.current = window.setTimeout(() => setShown(null), SHOWN_MS)
  }

  return (
    <>
      <button type="button" className="side-key side-key-up" aria-label="Volume +" onClick={() => press(1)} />
      <button type="button" className="side-key side-key-down" aria-label="Volume -" onClick={() => press(-1)} />
      {shown !== null && (
        <div className="side-volume" aria-hidden>
          <div className="side-volume-fill" style={{ height: `${Math.round(shown * 100)}%` }} />
          <svg viewBox="0 0 24 24" width="14" height="14">
            <path d="M4 9h4l5-4v14l-5-4H4Z" fill="currentColor" />
            {shown > 0 && <path d="M16 8.5a5 5 0 0 1 0 7" fill="none" stroke="currentColor" strokeWidth="2" />}
          </svg>
        </div>
      )}
    </>
  )
}
