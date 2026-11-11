import { useEffect, useRef, useState } from 'react'
import { usePhoneNotification } from '../cabin/state/store'
import { type PhoneNotification, postToHost } from '../shared/host'
import { chime, notificationText, waitingWith } from './notificationText'

/** How long a notification stays on the screen, and how long it takes to leave it (ms). */
const SHOWN_FOR = 4200
const LEAVING_FOR = 320

/**
 * The phone's notifications (owner 2026-11-11: as GTA RP's phones, for who streams on TikTok: "Mateus followed
 * you", a gift, each with its sound): a banner under the status bar, one at a time, each a few seconds. With the
 * phone put away the host shows only its top meanwhile (crates/host, the plugin's handset.rs), so the banner is
 * what's seen; it's told while one is on the screen.
 */
export function Notifications({ language }: { language: 'en' | 'pt' }) {
  const { latest, count } = usePhoneNotification()
  const waiting = useRef<PhoneNotification[]>([])
  const [shown, setShown] = useState<{ notification: PhoneNotification; id: number; leaving: boolean } | null>(null)
  const busy = useRef(false)
  const taken = useRef(0)
  const next = useRef(0)

  // The next one waiting, shown for its time and then let go; none left, the host is told.
  const showNext = useRef(() => {})
  showNext.current = () => {
    const notification = waiting.current.shift()
    if (!notification) {
      busy.current = false
      setShown(null)
      postToHost({ type: 'phoneBanner', showing: false })
      return
    }
    busy.current = true
    const id = ++next.current
    setShown({ notification, id, leaving: false })
    postToHost({ type: 'phoneBanner', showing: true })
    chime(notification.kind)
    setTimeout(() => setShown((now) => (now?.id === id ? { ...now, leaving: true } : now)), SHOWN_FOR)
    setTimeout(() => showNext.current(), SHOWN_FOR + LEAVING_FOR)
  }

  useEffect(() => {
    if (!latest || taken.current === count) return
    taken.current = count
    waiting.current = waitingWith(waiting.current, latest)
    if (!busy.current) showNext.current()
    // Each notification as it comes: `count` changes with every one (the same person twice is two).
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [count])

  if (!shown) return null
  const { notification } = shown
  return (
    <div
      key={shown.id}
      className={`ios-banner ios-banner-${notification.kind}${shown.leaving ? ' leaving' : ''}`}
      role="status"
    >
      <span className="ios-banner-icon ios-icon-tiktok" aria-hidden>
        <svg viewBox="0 0 24 24">
          <path d="M19.3 6.4a4.8 4.8 0 0 1-3.5-4v-.4h-3.4v13.7a2.9 2.9 0 1 1-3.9-2.7c.3 0 .7.1 1 .2v-3.5a6.4 6.4 0 0 0-1-.1 6.3 6.3 0 0 0-6.3 6.3c0 3.5 2.8 6.3 6.3 6.3a6.3 6.3 0 0 0 6.3-6.3V9a8.2 8.2 0 0 0 4.8 1.5V7a4.8 4.8 0 0 1-.3-.6Z" />
        </svg>
      </span>
      <span className="ios-banner-text">
        <span className="ios-banner-head">
          <b>TikTok</b>
          <i>{language === 'pt' ? 'agora' : 'now'}</i>
        </span>
        <span className="ios-banner-body">{notificationText(notification, language)}</span>
      </span>
    </div>
  )
}
