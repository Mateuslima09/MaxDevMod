import type { IconStyle } from '../shared/customize'
import { call, on } from './bridge'
import { useCallback, useEffect, useState } from 'react'

/** The player's settings. Mirrors `westplay_host::settings::Settings`; the app owns and saves them. */
export interface Settings {
  interaction: {
    /** The key that uses the screen. */
    key: KeyCode
    /** Held while using the screen, or pressed once to start and again to stop. */
    mode: InteractionMode
    /** Cursor speed multiplier, `SENSITIVITY.min`..`SENSITIVITY.max`. */
    sensitivity: number
  }
  /** What WestPlay does from the game, by the player's keys. */
  keybinds: Record<KeybindId, Binding>
  screen: {
    truckStrip: TruckStrip
    /** The split screen: the game's map squeezed into one half of its GPS screens (from the game's next start), or
     * off. */
    split: Split
  }
  /** How the screen looks (Customize): a wallpaper's id and one of the player's icon sets, by name; `null`:
   * none chosen (the cab's default wallpaper, WestPlay's own icons). */
  look: {
    wallpaper: string | null
    iconSet: string | null
    /** Light or dark icons, or as the screen is. */
    iconStyle: IconStyle
  }
  /** The graphics card the screen is drawn on: one the player chose (an `Adapter`'s id), or `null`:
   * WestPlay picks (the game's). */
  graphics: {
    gpu: string | null
  }
  /** The Modo Personagem: walking about on foot by the truck (F8 with it stopped), off at first; the mouse's speed
   * while walking, in percent of the game's (`CHARACTER_SENSITIVITY`). */
  character: {
    enabled: boolean
    sensitivity: number
    /** The key getting out and back in (a Windows virtual-key code: `CHARACTER_KEY` at first). */
    key: KeyCode
    /** Who walks: a place in the bridge's `CHARACTER_PEOPLE` (`CHARACTER_PEOPLE` here: their pictures). */
    person: number
    /** The key hints bar over the game (shown at first), and the key showing and hiding it there (0: none). */
    hints: boolean
    hintsKey: KeyCode
  }
}

/** Mirrors the bridge's `CHARACTER_SENSITIVITY` (the app corrects anything outside it). */
export const CHARACTER_SENSITIVITY = { min: 10, max: 200, step: 10 }
/** Mirrors the bridge's `CHARACTER_KEY` (F8) and `CHARACTER_PEOPLE`: WestPlay's own people, in its order, each with
 * their picture (`/characters/<prefix>.png`, rendered in Blender) and whether a man or a woman (their name); the
 * default the first. */
export const CHARACTER_KEY: KeyCode = 0x77
export const CHARACTER_PEOPLE = {
  people: [
    { picture: '/characters/wt.png', kind: 'man' },
    { picture: '/characters/wm.png', kind: 'woman' },
    { picture: '/characters/wc.png', kind: 'man' },
  ] as const,
  default: 0,
}

/** Mirrors `westplay_host::settings::TruckStrip`: the truck's own strip beside WestPlay, kept or not. */
export type TruckStrip = 'auto' | 'off'
/** Which half of the truck's screen the game's map is squeezed into, WestPlay in the other (`off`: the game's own). */
export type Split = 'off' | 'mapLeft' | 'mapRight'

/** Mirrors `westplay_host::settings::KeyCode`: a key of the keyboard or a button of the mouse, as Windows
 * numbers it (its virtual-key code: 0x05 the mouse's back button, 0x31 the 1 key, 0xA4 Left Alt). */
export type KeyCode = number

/** The keys the onboarding offers to hold (its keycaps), by the names their words have. */
export type KeyPreset = 'leftAlt' | 'rightCtrl' | 'mouseBack' | 'mouseForward'

export const KEY_PRESETS: Record<KeyPreset, KeyCode> = {
  leftAlt: 0xa4,
  rightCtrl: 0xa3,
  mouseBack: 0x05,
  mouseForward: 0x06,
}

/** The key WestPlay starts with: Left Alt. */
export const DEFAULT_KEY = KEY_PRESETS.leftAlt

/** Mirrors `westplay_host::settings::InteractionMode`. */
export type InteractionMode = 'hold' | 'toggle'

/** Mirrors `westplay_host::settings::Keybinds`: the playing music's volume up and down, play/pause, and the phone in
 * the game out or away. */
export type KeybindId = 'volumeUp' | 'volumeDown' | 'playPause' | 'phone' | 'character' | 'hints'| 'volumeDown' | 'playPause' | 'phone'

/** In a row as their default keys are (1, 2, 3), as on a stereo: volume down, play/pause, volume up; the phone, 4;
 * the phone's Personagem, 5; the key hints bar over the game, 6. */
export const KEYBIND_IDS: KeybindId[] = ['volumeDown', 'playPause', 'volumeUp', 'phone', 'character', 'hints']

/** Mirrors `westplay_host::settings::Binding`: a key, pressed alone or while using the screen (holding its
 * key, or toggled on). */
export interface Binding {
  key: KeyCode
  withInteractionKey: boolean
}

/** Mirrors `westplay_host::settings::Sensitivity` (the app corrects anything outside it). */
export const SENSITIVITY = { min: 0.5, max: 3, step: 0.1 }

export interface SettingsState {
  /** `null` until loaded. */
  settings: Settings | null
  /** Saves new settings; the app applies them at once. */
  update(settings: Settings): Promise<void>
  /** Why the last save failed, if it did. */
  error: string | null
}

/** The current settings, kept in step with every change (from this window or another). */
export function useSettings(): SettingsState {
  const [settings, setSettings] = useState<Settings | null>(null)
  const [error, setError] = useState<string | null>(null)

  useEffect(() => {
    let active = true
    call<Settings>('get_settings').then((loaded) => active && setSettings(loaded))
    const stop = on<Settings>('settings-changed', setSettings)
    return () => {
      active = false
      stop()
    }
  }, [])

  const update = useCallback(async (next: Settings) => {
    setSettings(next)
    try {
      setSettings(await call<Settings>('update_settings', { settings: next }))
      setError(null)
    } catch (e) {
      setError(String(e))
      setSettings(await call<Settings>('get_settings'))
    }
  }, [])

  return { settings, update, error }
}
