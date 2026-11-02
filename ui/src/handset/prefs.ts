import { useSyncExternalStore } from 'react'

/**
 * The phone's own settings (its Settings app), kept on the device showing it (the game's WebView2, or the
 * player's phone): its look, and the units it says distances and speeds in (owner 2026-11-02: "I want to choose
 * dark or light", "the game uses miles").
 */
export interface Prefs {
  /** `auto`: as the cab is (the headlights, the time of day, or the player's choice in the cab). */
  look: 'auto' | 'light' | 'dark'
  units: 'km' | 'mi'
}

const KEY = 'westplay-handset-prefs'
const DEFAULTS: Prefs = { look: 'auto', units: 'mi' }

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

/** Metres as the chosen unit, whole: "120 km", "75 mi". */
export function distance(metres: number, units: Prefs['units']): string {
  const km = metres / 1000
  return `${Math.round(units === 'mi' ? km / KM_PER_MILE : km).toLocaleString()} ${units}`
}

/** Metres per second as the chosen unit's speed, whole (no sign). */
export function speed(metresPerSecond: number, units: Prefs['units']): number {
  const kmh = Math.abs(metresPerSecond) * 3.6
  return Math.round(units === 'mi' ? kmh / KM_PER_MILE : kmh)
}

export function speedUnit(units: Prefs['units']): string {
  return units === 'mi' ? 'mph' : 'km/h'
}
