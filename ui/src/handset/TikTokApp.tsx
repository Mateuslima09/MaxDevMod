import { type ReactNode, useEffect, useLayoutEffect, useRef, useState } from 'react'
import { useTikTok } from '../cabin/state/store'
import { postToHost, type TikTokPage } from '../shared/host'
import { TIKTOK_WORDS, type TikTokWords } from './tiktokWords'

/** How long the app stays covered while TikTok starts, should the host never say (ms). */
const COVERED_AT_MOST = 6000

const TABS: { page: TikTokPage; word: keyof TikTokWords; glyph: ReactNode }[] = [
  { page: 'forYou', word: 'forYou', glyph: <path d="M3 11.2 12 3l9 8.2V21h-6v-6H9v6H3Z" /> },
  {
    page: 'following',
    word: 'following',
    glyph: (
      <>
        <circle cx="9" cy="8" r="3.6" />
        <path d="M2 20c.5-4 3.2-6.4 7-6.4s6.5 2.4 7 6.4Z" />
        <path d="M18.5 7v6M15.5 10h6" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
      </>
    ),
  },
  {
    page: 'explore',
    word: 'explore',
    glyph: (
      <>
        <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" strokeWidth="2" />
        <path d="m15.8 8.2-2.3 5.3-5.3 2.3 2.3-5.3Z" />
      </>
    ),
  },
  {
    page: 'live',
    word: 'live',
    glyph: (
      <>
        <rect x="3" y="6" width="18" height="13" rx="3" fill="none" stroke="currentColor" strokeWidth="2" />
        <path d="m10 9.5 5 3-5 3Z" />
        <path d="m8.5 2.5 3.5 3 3.5-3" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" />
      </>
    ),
  },
  {
    page: 'profile',
    word: 'profile',
    glyph: (
      <>
        <circle cx="12" cy="8" r="4" />
        <path d="M4 21c.6-4.6 3.8-7.2 8-7.2s7.4 2.6 8 7.2Z" />
      </>
    ),
  },
]

/**
 * TikTok in the phone (owner 2026-11-11: "the person can interact with the app as in TikTok"): TikTok's own site,
 * run by the host under the phone's page (crates/host tiktok.rs) and seen through this app, which is cut open
 * where it shows; the mouse over it is TikTok's page's (its feed scrolls with the wheel, its buttons are its own).
 * The app's own are only its tabs, which go to TikTok's pages, and a line saying what the account is up to. Signed
 * in under Accounts in WestPlay, as YouTube and Spotify are.
 */
export function TikTokApp({ language = 'pt', remote = false }: { language?: 'en' | 'pt'; remote?: boolean }) {
  const words = TIKTOK_WORDS[language]
  if (remote) return <p className="tiktok-remote">{words.remote}</p>
  return <TikTokInGame words={words} />
}

function TikTokInGame({ words }: { words: TikTokWords }) {
  const account = useTikTok()
  const hole = useRef<HTMLDivElement>(null)
  const [page, setPage] = useState<TikTokPage | null>('forYou')
  // Covered until the host says TikTok runs (its account, known or not yet), or long enough.
  const [waited, setWaited] = useState(false)
  useEffect(() => {
    const timer = setTimeout(() => setWaited(true), COVERED_AT_MOST)
    return () => clearTimeout(timer)
  }, [])
  const covered = account.signedIn === null && !waited

  // Where TikTok shows: wherever the app is cut open, in the phone's pixels, said again as it changes.
  useLayoutEffect(() => {
    const element = hole.current
    if (!element) return
    let last = ''
    const place = () => {
      const box = element.getBoundingClientRect()
      const command = {
        action: 'show' as const,
        x: Math.round(box.left),
        y: Math.round(box.top),
        width: Math.round(box.width),
        height: Math.round(box.height),
      }
      const now = JSON.stringify(command)
      if (now === last || command.width < 100 || command.height < 100) return
      last = now
      postToHost({ type: 'tiktok', command })
    }
    place()
    const watcher = new ResizeObserver(place)
    watcher.observe(element)
    return () => {
      watcher.disconnect()
      postToHost({ type: 'tiktok', command: { action: 'hide' } })
    }
  }, [])

  const note = account.signingIn
    ? words.signingIn
    : account.signedIn === false
      ? words.signedOut
      : account.live
        ? words.onLive
        : null

  return (
    <div className="tiktok-app">
      <div className="tiktok-hole" ref={hole}>
        {covered && (
          <div className="tiktok-cover">
            <span className="tiktok-spinner" aria-hidden />
            {words.loading}
          </div>
        )}
      </div>
      {note && <p className={`tiktok-note${account.live && !account.signingIn ? ' live' : ''}`}>{note}</p>}
      <nav className="tiktok-tabs">
        <button
          type="button"
          className="tiktok-tab"
          aria-label={words.back}
          onClick={() => {
            setPage(null)
            postToHost({ type: 'tiktok', command: { action: 'back' } })
          }}
        >
          <svg viewBox="0 0 24 24" aria-hidden>
            <path
              d="M15 4 7 12l8 8"
              fill="none"
              stroke="currentColor"
              strokeWidth="2.4"
              strokeLinecap="round"
              strokeLinejoin="round"
            />
          </svg>
          <span>{words.back}</span>
        </button>
        {TABS.map((tab) => (
          <button
            key={tab.page}
            type="button"
            className={`tiktok-tab${page === tab.page ? ' on' : ''}`}
            onClick={() => {
              setPage(tab.page)
              postToHost({ type: 'tiktok', command: { action: 'go', page: tab.page } })
            }}
          >
            <svg viewBox="0 0 24 24" aria-hidden>
              {tab.glyph}
            </svg>
            <span>{words[tab.word]}</span>
          </button>
        ))}
      </nav>
    </div>
  )
}
