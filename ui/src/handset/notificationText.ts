import type { PhoneNotification } from '../shared/host'

/** The most notifications waiting their turn: a busy LIVE's oldest are let go. */
const WAITING_MOST = 8

/** What a notification says, in the phone's language. */
export function notificationText(notification: PhoneNotification, language: 'en' | 'pt'): string {
  const { user, detail, count } = notification
  const times = count && count > 1 ? ` ×${count}` : ''
  if (language === 'pt') {
    switch (notification.kind) {
      case 'follow':
        return `${user} seguiu você`
      case 'gift':
        return `${user} enviou ${detail ?? 'um presente'}${times}`
      case 'share':
        return `${user} compartilhou sua LIVE`
      case 'subscribe':
        return `${user} se inscreveu`
    }
  }
  switch (notification.kind) {
    case 'follow':
      return `${user} followed you`
    case 'gift':
      return `${user} sent ${detail ?? 'a gift'}${times}`
    case 'share':
      return `${user} shared your LIVE`
    case 'subscribe':
      return `${user} subscribed`
  }
}

/** Takes its turn after those before it, the oldest let go once too many wait. */
export function waitingWith<T>(waiting: T[], next: T): T[] {
  return [...waiting, next].slice(-WAITING_MOST)
}

let audio: AudioContext | null = null

/**
 * A notification's sound, made here (no file to load): two bright notes for a follow, a share or a subscription;
 * a rising run of four for a gift.
 */
export function chime(kind: PhoneNotification['kind']): void {
  try {
    audio ??= new AudioContext()
    const context = audio
    if (context.state === 'suspended') void context.resume()
    const notes = kind === 'gift' ? [784, 988, 1175, 1568] : [1175, 1568]
    const step = kind === 'gift' ? 0.085 : 0.11
    const start = context.currentTime + 0.02
    notes.forEach((frequency, index) => {
      const at = start + index * step
      const oscillator = context.createOscillator()
      const gain = context.createGain()
      oscillator.type = 'sine'
      oscillator.frequency.value = frequency
      // A struck bell: there at once, ringing out.
      gain.gain.setValueAtTime(0.0001, at)
      gain.gain.exponentialRampToValueAtTime(0.22, at + 0.012)
      gain.gain.exponentialRampToValueAtTime(0.0001, at + 0.42)
      oscillator.connect(gain).connect(context.destination)
      oscillator.start(at)
      oscillator.stop(at + 0.45)
    })
  } catch {
    // No sound here (a phone's browser before it was touched): the notification still shows.
  }
}
