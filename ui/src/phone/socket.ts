import type { HostMessage, PageMessage } from '../shared/host'
import { makeInstallable } from './installable'

/**
 * The phone page's way to the host (crates/host remote.rs): a WebSocket to the PC it was served from, put where
 * the cab page has WebView2's own (`window.chrome.webview`), so the cab's code runs on the phone as it does in the
 * cab: the host's messages come in through it, and what the page posts goes out (the host takes only its playback
 * controls). Gone, it comes back a little later, and asks the host for the state again.
 *
 * The key is the address's fragment (the companion's QR code), kept on the phone for a visit without it.
 */

/** How long to wait before trying again, once the PC is gone (ms). */
const RETRY_MS = 2000
const KEPT_KEY = 'westplay-phone-key'

type Listener = (event: { data: HostMessage }) => void

/** Whether the phone is connected to the PC now, for the page to say. */
export const connection = {
  connected: false,
  listeners: new Set<(connected: boolean) => void>(),
}

function setConnected(connected: boolean) {
  connection.connected = connected
  for (const listener of connection.listeners) listener(connected)
}

/** Puts the stand-in in place, before any of the cab's code runs. Returns whether it could (it needs the key). */
export function installPhoneSocket(): boolean {
  const key = keyOf()
  if (!key) return false
  makeInstallable(key)
  const listeners = new Set<Listener>()
  const waiting: string[] = []
  let socket: WebSocket | null = null
  let ever = false

  const connect = () => {
    const protocol = location.protocol === 'https:' ? 'wss:' : 'ws:'
    const opened = new WebSocket(`${protocol}//${location.host}/ws?k=${encodeURIComponent(key)}`)
    socket = opened
    opened.onopen = () => {
      setConnected(true)
      // Again after a drop: the host sends what the page missed.
      if (ever) opened.send(JSON.stringify({ type: 'ready' }))
      ever = true
      for (const message of waiting.splice(0)) opened.send(message)
    }
    opened.onmessage = (event) => {
      let data: HostMessage
      try {
        data = JSON.parse(String(event.data)) as HostMessage
      } catch {
        return
      }
      for (const listener of listeners) listener({ data })
    }
    opened.onclose = () => {
      if (socket === opened) socket = null
      setConnected(false)
      setTimeout(connect, RETRY_MS)
    }
  }

  window.westplayPhone = true
  window.chrome = {
    webview: {
      addEventListener: (_type, listener) => listeners.add(listener),
      removeEventListener: (_type, listener) => listeners.delete(listener),
      postMessage: (message: PageMessage) => {
        const json = JSON.stringify(message)
        if (socket?.readyState === WebSocket.OPEN) {
          socket.send(json)
          // A volume set here shows on the truck's screen too, as a keybind's does (owner 2026-11-01).
          const volume = volumeSet(message)
          if (volume !== null) socket.send(JSON.stringify({ type: 'volumeShown', volume } satisfies PageMessage))
        } else if (message.type === 'ready') waiting.push(json)
      },
    },
  }
  connect()
  return true
}

/** The volume (0 to 1) a player's command sets, if it's one that sets it. */
function volumeSet(message: PageMessage): number | null {
  // A player's command is an object (the truck's commands are named, by a string).
  if (!('command' in message) || typeof message.command !== 'object') return null
  const command: { action: string; percent?: number } = message.command
  return command.action === 'volume' && typeof command.percent === 'number' ? command.percent / 100 : null
}

/** The key: the address's fragment (kept for next time), else the one kept. */
function keyOf(): string | null {
  const given = location.hash.slice(1)
  try {
    if (given) localStorage.setItem(KEPT_KEY, given)
    return given || localStorage.getItem(KEPT_KEY)
  } catch {
    return given || null
  }
}
