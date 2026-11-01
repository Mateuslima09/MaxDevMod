import { useEffect, useState, useSyncExternalStore } from 'react'
import { Dashboard } from '../cabin/dashboard/Dashboard'
import { useLook, useLookChanges } from '../cabin/state/hooks'
import { connectHost, useLive } from '../cabin/state/store'
import { type Language, languageOf } from '../cabin/state/strings'
import { followTimeZone } from '../cabin/state/timeZone'
import { PhoneVolume } from './PhoneVolume'
import { connection } from './socket'

/**
 * The phone page (owner 2026-11-01): the cab's Dashboard on the player's phone, beside the truck's screen, not
 * instead of it. The map following the truck, what plays with its controls and volume, and the delivery, from
 * the same host messages the cab has (phone/socket.ts). Nothing of the cab's own doing runs here (pausing with
 * the game, the keybinds, one player at a time): the cab does it, once.
 */
export function PhoneApp({ connects }: { connects: boolean }) {
  useEffect(() => connectHost(), [])
  // The clock where the truck is, as the cab's.
  useEffect(() => followTimeZone(), [])
  const language = languageOf(useLive((state) => state.gameLanguage))
  useEffect(() => {
    document.documentElement.lang = language
  }, [language])
  // Dark or light as the cab is (the headlights, the time of day, or the player's choice), its colours the shell's.
  useLookChanges()
  const { appearance } = useLook()
  const connected = useConnected()
  const full = useFullScreen()
  const words = WORDS[language]

  return (
    <main className={`phone shell appearance-${appearance}`}>
      <header className="phone-bar">
        <span className="phone-name">WestPlay</span>
        <span className={`phone-link${connected ? ' on' : ''}`}>
          {!connects ? words.noKey : connected ? words.connected : words.connecting}
        </span>
        <PhoneVolume label={words.volume} />
        {document.fullscreenEnabled && (
          <button
            type="button"
            className="phone-full"
            aria-label={full ? words.leaveFullScreen : words.fullScreen}
            onClick={() => void (full ? document.exitFullscreen() : goFullScreen())}
          >
            <svg viewBox="0 0 24 24" width="20" height="20" aria-hidden>
              <path
                d={full ? 'M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5' : 'M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5'}
                fill="none"
                stroke="currentColor"
                strokeWidth="2"
              />
            </svg>
          </button>
        )}
      </header>
      <Dashboard />
    </main>
  )
}

/** The page full screen, without the browser's bar (owner 2026-11-01: "the browser's bar stays, sideways"). */
function goFullScreen(): Promise<void> {
  return document.documentElement.requestFullscreen({ navigationUI: 'hide' }).catch(() => {})
}

/** Whether the page is full screen now. The first touch makes it so (a browser goes full screen only for one). */
function useFullScreen(): boolean {
  const [full, setFull] = useState(() => document.fullscreenElement !== null)
  useEffect(() => {
    const follow = () => setFull(document.fullscreenElement !== null)
    const first = () => {
      if (document.fullscreenEnabled && document.fullscreenElement === null) void goFullScreen()
    }
    document.addEventListener('fullscreenchange', follow)
    window.addEventListener('pointerup', first, { once: true })
    return () => {
      document.removeEventListener('fullscreenchange', follow)
      window.removeEventListener('pointerup', first)
    }
  }, [])
  return full
}

/** Whether the phone is connected to the PC now. */
function useConnected(): boolean {
  return useSyncExternalStore(
    (changed) => {
      connection.listeners.add(changed)
      return () => {
        connection.listeners.delete(changed)
      }
    },
    () => connection.connected,
  )
}

interface Words {
  connected: string
  connecting: string
  noKey: string
  volume: string
  fullScreen: string
  leaveFullScreen: string
}

const WORDS: Record<Language, Words> = {
  en: {
    connected: 'Connected to your PC',
    connecting: 'Connecting to your PC…',
    noKey: 'Scan the QR code in WestPlay on your PC',
    volume: 'Volume',
    fullScreen: 'Full screen',
    leaveFullScreen: 'Leave full screen',
  },
  pt: {
    connected: 'Conectado ao seu PC',
    connecting: 'Conectando ao seu PC…',
    noKey: 'Escaneie o QR Code no WestPlay do seu PC',
    volume: 'Volume',
    fullScreen: 'Tela cheia',
    leaveFullScreen: 'Sair da tela cheia',
  },
  tr: {
    connected: 'Bilgisayarınıza bağlı',
    connecting: 'Bilgisayarınıza bağlanıyor…',
    noKey: 'Bilgisayarınızdaki WestPlay’de QR kodu okutun',
    volume: 'Ses',
    fullScreen: 'Tam ekran',
    leaveFullScreen: 'Tam ekrandan çık',
  },
}
