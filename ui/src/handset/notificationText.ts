import type { GameNotification, PhoneNotification } from '../shared/host'
import { offenceName } from './offences'
import { type Prefs, formatMoney } from './prefs'

/** The most notifications waiting their turn: a busy LIVE's oldest are let go. */
const WAITING_MOST = 8

/** What a notification says, in the phone's language (the game's money in the currency chosen in Nupay). */
export function notificationText(
  notification: PhoneNotification,
  language: 'en' | 'pt',
  currency: Prefs['currency'] = 'auto',
): string {
  if (notification.app !== 'tiktok') return gameText(notification, language, currency)
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

/** The app's name a notification is of, as the banner's head says it. */
export function notificationApp(notification: PhoneNotification, language: 'en' | 'pt'): string {
  switch (notification.app) {
    case 'tiktok':
      return 'TikTok'
    case 'nupay':
      return 'Nupay'
    case 'truck':
      return language === 'pt' ? 'Caminhão' : 'Truck'
    case 'freight':
      return language === 'pt' ? 'Fretes' : 'Jobs'
  }
}

const PARTS = {
  en: { engine: 'engine', transmission: 'transmission', cabin: 'cab', chassis: 'chassis', wheels: 'wheels' },
  pt: { engine: 'motor', transmission: 'câmbio', cabin: 'cabine', chassis: 'chassi', wheels: 'rodas' },
} as const

/** The game's and MaxDevMod's notifications (crates/host alerts.rs), worded. */
function gameText(notification: GameNotification, language: 'en' | 'pt', currency: Prefs['currency']): string {
  const { kind, amount = 0, value = 0, detail, count } = notification
  const money = formatMoney(Math.abs(amount), notification.currency ?? 'USD', currency, language)
  const times = count && count > 1 ? ` (×${count})` : ''
  const about = detail ? ` · ${detail}` : ''
  const part = PARTS[language][detail as keyof (typeof PARTS)['en']] ?? detail ?? ''
  const owed = `${detail ?? ''} ${(notification.total ?? 0).toLocaleString(language === 'pt' ? 'pt-BR' : 'en-US', {
    minimumFractionDigits: 2,
    maximumFractionDigits: 2,
  })}`.trim()
  if (language === 'pt') {
    switch (kind) {
      case 'payAttendant':
        return `Pague ${owed} ao frentista: ele está na frente do caminhão, aperte E perto dele`
      case 'delivered':
        return `Pix recebido: ${money} pelo frete${about}`
      case 'cancelled':
        return `Multa por cancelar o frete: ${money}${about}`
      case 'fined':
        return `Você levou uma multa de ${money}${detail ? ` · ${offenceName(detail, 'pt')}` : ''}`
      case 'toll':
        return `Pedágio pago: ${money}${times}`
      case 'ferry':
        return `Balsa paga: ${money}${about}`
      case 'train':
        return `Trem pago: ${money}${about}`
      case 'fuel':
        return `Combustível pago: ${money}${about}`
      case 'otherIn':
        return `Entrada na conta: ${money}`
      case 'otherOut':
        return `Saída da conta: ${money} (oficina, diesel, compras)`
      case 'fuelLow':
        return value > 0 ? `Combustível na reserva: dá uns ${value} km` : 'Combustível na reserva: abasteça logo'
      case 'damage':
        return `Caminhão com ${value}% de dano${part ? ` (${part})` : ''}. Passe na oficina.`
      case 'crash':
        return `Batida! O caminhão está com ${value}% de dano${part ? ` (${part})` : ''}`
      case 'repaired':
        return 'Caminhão consertado ✔'
      case 'rest':
        return `Hora de descansar em ${value} min: procure um lugar para dormir`
      case 'restNow':
        return 'Você está cansado: pare para dormir'
      case 'jobNew':
        return `Novo frete${about}${amount > 0 ? ` · ${money}` : ''}`
      case 'jobDeadline':
        return `Faltam ${value} min para o prazo da entrega`
      case 'jobLate':
        return 'A entrega está atrasada'
    }
  }
  switch (kind) {
    case 'payAttendant':
      return `Pay ${owed} to the attendant: they're in front of the truck, press E by them`
    case 'delivered':
      return `Payment received: ${money} for the job${about}`
    case 'cancelled':
      return `Job cancelled, penalty: ${money}${about}`
    case 'fined':
      return `You were fined ${money}${detail ? ` · ${offenceName(detail, 'en')}` : ''}`
    case 'toll':
      return `Toll paid: ${money}${times}`
    case 'ferry':
      return `Ferry paid: ${money}${about}`
    case 'train':
      return `Train paid: ${money}${about}`
    case 'fuel':
      return `Fuel paid: ${money}${about}`
    case 'otherIn':
      return `Money in: ${money}`
    case 'otherOut':
      return `Money out: ${money} (repairs, fuel, purchases)`
    case 'fuelLow':
      return value > 0 ? `Low fuel: about ${value} km left` : 'Low fuel: fill up soon'
    case 'damage':
      return `Truck at ${value}% damage${part ? ` (${part})` : ''}. Visit a service.`
    case 'crash':
      return `Crash! The truck is at ${value}% damage${part ? ` (${part})` : ''}`
    case 'repaired':
      return 'Truck repaired ✔'
    case 'rest':
      return `Rest due in ${value} min: find a place to sleep`
    case 'restNow':
      return 'You’re tired: stop to sleep'
    case 'jobNew':
      return `New job${about}${amount > 0 ? ` · ${money}` : ''}`
    case 'jobDeadline':
      return `${value} min left to deliver on time`
    case 'jobLate':
      return 'The delivery is late'
  }
}

/** Whether a notification is a warning (a low, falling sound, its text in bold): a fine, damage, a late job. */
export function isWarning(notification: PhoneNotification): boolean {
  return (
    notification.app !== 'tiktok' &&
    ['fined', 'fuelLow', 'damage', 'crash', 'restNow', 'jobLate', 'cancelled', 'payAttendant'].includes(
      notification.kind,
    )
  )
}

/** Takes its turn after those before it, the oldest let go once too many wait. */
export function waitingWith<T>(waiting: T[], next: T): T[] {
  return [...waiting, next].slice(-WAITING_MOST)
}

let audio: AudioContext | null = null

/**
 * A notification's sound, made here (no file to load): two bright notes for a follow, a share or a subscription;
 * a rising run of four for a gift or money coming in; two falling low ones for a warning.
 */
export function chime(notification: PhoneNotification): void {
  const kind = notification.kind
  const warning = isWarning(notification)
  const rising = kind === 'gift' || kind === 'delivered' || kind === 'otherIn'
  try {
    audio ??= new AudioContext()
    const context = audio
    if (context.state === 'suspended') void context.resume()
    const notes = warning ? [880, 587] : rising ? [784, 988, 1175, 1568] : [1175, 1568]
    const step = warning ? 0.16 : rising ? 0.085 : 0.11
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
