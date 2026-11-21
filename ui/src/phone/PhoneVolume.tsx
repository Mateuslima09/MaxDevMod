import { useEffect, useRef, useState } from 'react'
import { appleMusicPlayer } from '../cabin/apps/music/pauseWithGame'
import { useAppleMusic } from '../cabin/apps/music/appleMusic'
import type { MusicPlayer } from '../cabin/apps/musicWithGame'
import { spotifyPlayer } from '../cabin/apps/spotify/pauseWithGame'
import { useSpotify } from '../cabin/apps/spotify/spotify'
import { youtubePlayer } from '../cabin/apps/youtube/pauseWithGame'
import { useYouTube } from '../cabin/apps/youtube/youtube'
import { youtubeMusicPlayer } from '../cabin/apps/youtubeMusic/pauseWithGame'
import { useYouTubeMusic } from '../cabin/apps/youtubeMusic/youtubeMusic'
import { type NowPlayingSource } from '../cabin/dashboard/nowPlaying'
import { useNowPlaying } from '../cabin/dashboard/useNowPlaying'

export const PLAYERS: Record<NowPlayingSource, MusicPlayer> = {
  video: youtubePlayer,
  spotify: spotifyPlayer,
  appleMusic: appleMusicPlayer,
  youtubeMusic: youtubeMusicPlayer,
}

/** How often a slider being dragged sets the volume, at most (ms): each is a command to the player on the PC. */
const SEND_EVERY_MS = 150

/**
 * The phone's volume (owner 2026-11-01: "add a volume control"): of the player Now Playing shows, as the cab's
 * keybinds set it, its value as the player last said. None while nothing is playing or the player has none to set.
 */
export function PhoneVolume({ label }: { label: string }) {
  const { showing } = useNowPlaying()
  const player = showing ? PLAYERS[showing] : null
  // Drawn again as the players say how they play: their volume is in it.
  useSpotify((state) => state.playback)
  useAppleMusic((state) => state.playback)
  useYouTubeMusic((state) => state.playback)
  useYouTube((state) => state.state)
  const told = player?.volume() ?? null
  // While dragged: the slider's own, not the player's (which comes back a moment later).
  const [dragged, setDragged] = useState<number | null>(null)
  const sending = useRef<Sending>({ sent: 0, pending: null })
  useEffect(() => () => window.clearTimeout(sending.current.pending ?? undefined), [])

  if (!player || told === null) return null
  const set = (share: number, now: boolean) => send(sending.current, player, share, now)
  const shown = dragged ?? told
  return (
    <label className="phone-volume">
      <svg viewBox="0 0 24 24" width="18" height="18" aria-hidden>
        <path d="M4 9h4l5-4v14l-5-4H4Z" fill="currentColor" />
        <path d="M16 8.5a5 5 0 0 1 0 7M18.5 6a8.5 8.5 0 0 1 0 12" fill="none" stroke="currentColor" strokeWidth="2" />
      </svg>
      <input
        type="range"
        min={0}
        max={100}
        value={Math.round(shown * 100)}
        aria-label={label}
        onChange={(event) => {
          const share = Number(event.target.value) / 100
          setDragged(share)
          set(share, false)
        }}
        onPointerUp={() => {
          if (dragged !== null) set(dragged, true)
          setDragged(null)
        }}
      />
    </label>
  )
}

/** When a dragged slider last set the volume, and the setting waiting for its turn. */
interface Sending {
  sent: number
  pending: number | null
}

/** Sets `player`'s volume, now or once [`SEND_EVERY_MS`] has passed since the last (the latest only). */
function send(sending: Sending, player: MusicPlayer, share: number, now: boolean) {
  window.clearTimeout(sending.pending ?? undefined)
  const wait = SEND_EVERY_MS - (Date.now() - sending.sent)
  if (now || wait <= 0) {
    sending.sent = Date.now()
    player.setVolume(share)
  } else {
    sending.pending = window.setTimeout(() => send(sending, player, share, true), wait)
  }
}
