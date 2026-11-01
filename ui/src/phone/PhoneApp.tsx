import { useEffect, useState } from 'react'
import { Dashboard } from '../cabin/dashboard/Dashboard'
import { useLook, useLookChanges } from '../cabin/state/hooks'
import { connectHost, useLive } from '../cabin/state/store'
import { type Language, languageOf } from '../cabin/state/strings'
import { followTimeZone } from '../cabin/state/timeZone'
import { connection } from './socket'

/**
 * The phone page (owner 2026-11-01): the cab's Dashboard on the player's phone, beside the truck's screen, not
 * instead of it. The map following the truck, what plays with its controls, and the delivery, from the same
 * host messages the cab has (phone/socket.ts). Nothing of the cab's own doing runs here (pausing with the game,
 * the keybinds, one player at a time): the cab does it, once.
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
  const words = WORDS[language]

  return (
    <main className={`phone shell appearance-${appearance}`}>
      <header className="phone-bar">
        <span className="phone-name">WestPlay</span>
        <span className={`phone-link${connected ? ' on' : ''}`}>
          {!connects ? words.noKey : connected ? words.connected : words.connecting}
        </span>
      </header>
      <Dashboard />
    </main>
  )
}

/** Whether the phone is connected to the PC now. */
function useConnected(): boolean {
  const [connected, setConnected] = useState(connection.connected)
  useEffect(() => {
    connection.listeners.add(setConnected)
    // It may have connected between the first render and now.
    setConnected(connection.connected)
    return () => {
      connection.listeners.delete(setConnected)
    }
  }, [])
  return connected
}

const WORDS: Record<Language, { connected: string; connecting: string; noKey: string }> = {
  en: {
    connected: 'Connected to your PC',
    connecting: 'Connecting to your PC…',
    noKey: 'Scan the QR code in WestPlay on your PC',
  },
  pt: {
    connected: 'Conectado ao seu PC',
    connecting: 'Conectando ao seu PC…',
    noKey: 'Escaneie o QR Code no WestPlay do seu PC',
  },
  tr: {
    connected: 'Bilgisayarınıza bağlı',
    connecting: 'Bilgisayarınıza bağlanıyor…',
    noKey: 'Bilgisayarınızdaki WestPlay’de QR kodu okutun',
  },
}
