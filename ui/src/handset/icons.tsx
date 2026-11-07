import type { ReactNode } from 'react'

/** The phone's apps (HandsetApp.tsx). */
export type AppId =
  | 'maps'
  | 'music'
  | 'youtubeMusic'
  | 'spotify'
  | 'commands'
  | 'freight'
  | 'truck'
  | 'settings'
  | 'nupay'
  | 'services'
  | 'character'

/**
 * The apps that are the cab's too: their real icons (owner 2026-11-03: "the apps' real logos"), the cab's light
 * set drawn once into plain images (public/icons/phone, 180 px): drawn from their SVGs, with their blurred
 * shadows, a phone's browser stalled on them (the owner's: its dock and Settings' icon never came).
 */
const REAL: Partial<Record<AppId, string>> = {
  maps: 'maps',
  music: 'music',
  youtubeMusic: 'youtube-music',
  spotify: 'spotify',
  settings: 'settings',
}

/** An app's icon as iOS draws them (`small`: a list row's): its real one, or for the phone's own apps a rounded
 * square of its colours, its glyph in white. */
export function AppIcon({ id, small = false }: { id: AppId; small?: boolean }) {
  const file = REAL[id]
  return file ? (
    <RealIcon file={file} small={small} />
  ) : (
    <span className={`ios-icon ios-icon-${id}${small ? ' small' : ''}`} aria-hidden>
      <svg viewBox="0 0 24 24">{GLYPHS[id]}</svg>
    </span>
  )
}

/** A real icon in its colours, as the apps' own, whatever the phone's look. */
function RealIcon({ file, small }: { file: string; small: boolean }) {
  const src = `/icons/phone/${file}.png`
  return (
    <span className={`ios-icon ios-icon-real${small ? ' small' : ''}`} aria-hidden>
      <img src={src} alt="" draggable={false} />
    </span>
  )
}

/** A small rounded square with a glyph, as iOS Settings' rows have (`tone`: its colour). */
export function RowIcon({ tone, children }: { tone: string; children: ReactNode }) {
  return (
    <span className="ios-row-icon" style={{ background: tone }} aria-hidden>
      <svg viewBox="0 0 24 24">{children}</svg>
    </span>
  )
}

const STROKE = {
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 2,
  strokeLinecap: 'round',
  strokeLinejoin: 'round',
} as const

const GLYPHS: Record<AppId, ReactNode> = {
  character: (
    <>
      <circle cx="12" cy="7.5" r="3.5" />
      <path d="M5 20.5c.6-4.2 3.3-6.8 7-6.8s6.4 2.6 7 6.8Z" />
    </>
  ),
  services: <path d="M14.7 6.3a4 4 0 0 0-5.4 5l-6 6a1.4 1.4 0 0 0 2 2l6-6a4 4 0 0 0 5-5.4l-2.6 2.6-2-.6-.6-2Z" />,
  nupay: (
    <>
      <rect x="2.5" y="5.5" width="19" height="13" rx="3" {...STROKE} />
      <path d="M2.5 10h19" {...STROKE} />
      <path d="M6 14.5h4" {...STROKE} />
    </>
  ),
  maps: (
    <>
      <path d="M3 6.5 9 4l6 2.5L21 4v13.5L15 20l-6-2.5-6 2.5Z" {...STROKE} strokeWidth={1.6} opacity={0.55} />
      <path d="M12 6.5 16.5 17 12 14.6 7.5 17Z" />
    </>
  ),
  music: (
    <>
      <path d="M9 17.5V5.5l11-2v12" {...STROKE} />
      <circle cx="6.5" cy="17.5" r="2.7" />
      <circle cx="17.5" cy="15.5" r="2.7" />
    </>
  ),
  youtubeMusic: (
    <>
      <circle cx="12" cy="12" r="8.2" {...STROKE} strokeWidth={1.8} />
      <path d="M10 8.6v6.8l5.8-3.4Z" />
    </>
  ),
  spotify: (
    <path
      d="M5 9.2c4.5-1.4 9.5-1 13.5 1.2M6 12.8c3.8-1.1 7.8-.7 11 1M7.2 16.2c3-.8 6-.5 8.6.8"
      {...STROKE}
      strokeWidth={2.2}
    />
  ),
  commands: (
    <>
      <rect x="3.5" y="3.5" width="7.5" height="7.5" rx="2.2" />
      <rect x="13" y="3.5" width="7.5" height="7.5" rx="3.75" opacity={0.75} />
      <rect x="3.5" y="13" width="7.5" height="7.5" rx="3.75" opacity={0.75} />
      <rect x="13" y="13" width="7.5" height="7.5" rx="2.2" />
    </>
  ),
  freight: (
    <>
      <path d="M3.5 7.5 12 3.5l8.5 4v9L12 20.5l-8.5-4Z" {...STROKE} />
      <path d="M3.5 7.5 12 11.5l8.5-4M12 11.5v9" {...STROKE} />
    </>
  ),
  truck: (
    <>
      <path d="M2.5 6.5h11v9h-11ZM13.5 9h4l3 3.5v3h-7Z" />
      <circle cx="6.5" cy="17" r="2" stroke="currentColor" strokeWidth={1.5} fill="#1c7ed6" />
      <circle cx="17" cy="17" r="2" stroke="currentColor" strokeWidth={1.5} fill="#1c7ed6" />
    </>
  ),
  settings: (
    <path
      d="M12 8.6a3.4 3.4 0 1 0 0 6.8 3.4 3.4 0 0 0 0-6.8Zm8.3 3.4 1.9-1.5-1.9-3.4-2.3.8a7.3 7.3 0 0 0-2-1.1L15.6 4.4h-3.9l-.4 2.4a7.3 7.3 0 0 0-2 1.1l-2.3-.8-1.9 3.4 1.9 1.5a7.3 7.3 0 0 0 0 2.2L5.1 15.7l1.9 3.4 2.3-.8a7.3 7.3 0 0 0 2 1.1l.4 2.4h3.9l.4-2.4a7.3 7.3 0 0 0 2-1.1l2.3.8 1.9-3.4-1.9-1.5a7.3 7.3 0 0 0 0-2.2Z"
      fillRule="evenodd"
    />
  ),
}
