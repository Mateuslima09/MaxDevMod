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
