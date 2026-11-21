import { useSyncExternalStore } from 'react'
import type { AlertGroup } from '../shared/host'

/**
 * The phone's own settings (its Settings app), kept on the device showing it (the game's WebView2, or the
 * player's phone): its look, and the units it says distances and speeds in (owner 2026-11-02: "I want to choose
 * dark or light", "the game uses miles").
 */
export interface Prefs {
  /** `auto`: as the cab is (the headlights, the time of day, or the player's choice in the cab). */
  look: 'auto' | 'light' | 'dark'
  units: 'km' | 'mi'
  /** The home screen's wallpaper, the same whatever the look (owner 2026-11-03: "so it doesn't keep changing"). */
  wallpaper: Wallpaper
  /** What Nupay shows the money in (owner 2026-11-02: "dollars, reais and euros"): `auto`, the game's own. */
  currency: 'auto' | Currency
  /** Whether the account's TikTok LIVE is watched for notifications (owner 2026-11-11: who followed, who sent a
   * gift). */
  tiktokNotifications: boolean
  /** Whether TikTok was signed in when the host last said: it's then run as the phone loads, for the notifications,
   * without its app being opened first. Only whether, nothing of the account. */
  tiktokSignedIn: boolean
  /** The cargo's papers and coupling the trailer by hand (owner 2026-11-11: the nota fiscal signed with the clerk
   * before coupling, the recipient's signature before delivering): on unless the player turns it off. */
  freightPapers: boolean
  /** Refuelling by hand (owner 2026-11-11: at a pump, the tank is filled on foot at the truck's side): on unless
   * the player turns it off. */
  manualRefuel: boolean
  /** The game's notifications' groups turned off (owner 2026-11-20: payments, fines, the truck, the job): all on
   * unless the player turns one off. */
  alertsOff: AlertGroup[]
}

const KEY = 'westplay-handset-prefs'
const DEFAULTS: Prefs = {
  look: 'auto',
  units: 'mi',
  wallpaper: 'aurora',
  currency: 'auto',
  tiktokNotifications: true,
  tiktokSignedIn: false,
  freightPapers: true,
  manualRefuel: true,
  alertsOff: [],
}

export type Currency = 'USD' | 'BRL' | 'EUR'

/**
 * How much of each a dollar is: fixed, roughly as they were in late 2026, not followed live (the phone may have no
 * internet, and the game's money isn't real money anyway). Nupay says it's converted when it is.
 */
const PER_DOLLAR: Record<Currency, number> = { USD: 1, BRL: 5.4, EUR: 0.86 }

/** `amount` of `from` in `to`, at the fixed rates. */
export function convert(amount: number, from: Currency, to: Currency): number {
  return from === to ? amount : (amount / PER_DOLLAR[from]) * PER_DOLLAR[to]
}

/** The currency the phone shows the game's money in: the one chosen in Nupay, or the game's own (`auto`). */
export function shownCurrency(chosen: Prefs['currency'], game: Currency): Currency {
  return chosen === 'auto' ? game : chosen
}

/**
 * The game's `amount` (in `game`, its currency) as the phone shows money (owner 2026-11-03: the job's pay in dollars
 * whatever Nupay's currency was): converted to the currency chosen, whole, as the player's language writes it;
 * `signed`: with + or −.
 */
export function formatMoney(
  amount: number,
  game: Currency,
  chosen: Prefs['currency'],
  language: 'en' | 'pt',
  signed = false,
): string {
  const currency = shownCurrency(chosen, game)
  const shown = convert(amount, game, currency)
  const text = Math.abs(shown).toLocaleString(language === 'pt' ? 'pt-BR' : 'en-US', {
    style: 'currency',
    currency,
    maximumFractionDigits: 0,
  })
  if (!signed) return shown < 0 ? `−${text}` : text
  return `${shown < 0 ? '−' : '+'}${text}`
}

/** The home screen's wallpapers to choose from. */
export const WALLPAPERS = ['aurora', 'ocean', 'sunset'] as const
export type Wallpaper = (typeof WALLPAPERS)[number]

let prefs: Prefs = read()
const listeners = new Set<() => void>()

function read(): Prefs {
  try {
    return { ...DEFAULTS, ...(JSON.parse(localStorage.getItem(KEY) ?? '{}') as Partial<Prefs>) }
  } catch {
    return DEFAULTS
  }
}

export function setPrefs(change: Partial<Prefs>): void {
  prefs = { ...prefs, ...change }
  try {
    localStorage.setItem(KEY, JSON.stringify(prefs))
  } catch {
    // Kept for this visit only.
  }
  for (const listener of listeners) listener()
}

export function usePrefs(): Prefs {
  return useSyncExternalStore(
    (changed) => {
      listeners.add(changed)
      return () => listeners.delete(changed)
    },
    () => prefs,
  )
}

const KM_PER_MILE = 1.609344

const FEET_PER_METRE = 3.28084

/** Metres as the chosen unit: "120 km", "75 mi", "4.2 mi"; under one, in metres or feet ("250 m", "820 ft"),
 * not rounded away to nothing (owner 2026-11-02: the job said 0 mi with the map at 250 m). */
export function distance(metres: number, units: Prefs['units']): string {
  const km = metres / 1000
  const far = units === 'mi' ? km / KM_PER_MILE : km
  if (far < 1) {
    const near = units === 'mi' ? metres * FEET_PER_METRE : metres
    return `${(Math.round(near / 10) * 10).toLocaleString()} ${units === 'mi' ? 'ft' : 'm'}`
  }
  const digits = far < 10 ? 1 : 0
  return `${far.toLocaleString(undefined, { maximumFractionDigits: digits })} ${units}`
}

/** Metres per second as the chosen unit's speed, whole (no sign). */
export function speed(metresPerSecond: number, units: Prefs['units']): number {
  const kmh = Math.abs(metresPerSecond) * 3.6
  return Math.round(units === 'mi' ? kmh / KM_PER_MILE : kmh)
}

export function speedUnit(units: Prefs['units']): string {
  return units === 'mi' ? 'mph' : 'km/h'
}
