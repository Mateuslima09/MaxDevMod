import { lazy } from 'react'

/*
 * The phone's heavy parts, loaded only once opened (owner 2026-11-03: "it opens stalling, the whole phone"): the
 * map (its engine, over a megabyte of script) and the cab's music apps, with their keyboard. Loaded with the page,
 * the phone's report showed it held for most of a second at a time for its first seconds.
 */

export const MapView = lazy(() => import('../cabin/dashboard/MapView').then((m) => ({ default: m.MapView })))

export const YouTubeMusicApp = lazy(() =>
  import('../cabin/apps/youtubeMusic/YouTubeMusicApp').then((m) => ({ default: m.YouTubeMusicApp })),
)

export const SpotifyApp = lazy(() =>
  import('../cabin/apps/spotify/SpotifyApp').then((m) => ({ default: m.SpotifyApp })),
)

export const Keyboard = lazy(() => import('../cabin/keyboard/Keyboard').then((m) => ({ default: m.Keyboard })))
