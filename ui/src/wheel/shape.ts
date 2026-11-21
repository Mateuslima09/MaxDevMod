/**
 * The phone wheel's settings and how they shape what the phone feels into what the truck is given (owner
 * 2026-11-19: "options to raise and lower sensitivity, smoothness, accessibility... so a player can change what
 * they want"). Pure, so it's tested (shape.test.ts); the page (main.ts) feeds it the gyroscope and the pedals.
 */

export type PedalMode = 'touch' | 'tilt' | 'onoff'

export interface WheelSettings {
  /** How much the truck turns per degree of phone: a multiplier, 0.5 to 2. */
  sensitivity: number
  /** Degrees of phone, each way, for full lock (before `sensitivity`): 45 to 180. */
  range: number
  /** The steering filter's strength: 0 (none, fastest) to 0.9 (smoothest). */
  smoothing: number
  /** Degrees of phone around the centre that steer nothing: 0 to 10. */
  deadzone: number
  /** The steering's response: 0 linear to 1 progressive (gentle in the middle, quick near full lock). */
  curve: number
  invert: boolean
  /** The pedals': a multiplier (0.5 to 2, more reaches full sooner), and a curve as the steering's. */
  throttleSensitivity: number
  throttleCurve: number
  brakeSensitivity: number
  brakeCurve: number
  /** The pedals by the touch's height, by tilting the phone (forward throttle, back brake), or all-or-nothing. */
  pedalMode: PedalMode
  /** Degrees of tilt for a full pedal (tilt mode): 10 to 45. */
  tiltRange: number
  tiltInvert: boolean
  vibration: boolean
  /** 1 light to 3 strong. */
  vibrationStrength: number
  /** The pedals swapped: throttle on the left, brake on the right. */
  leftHanded: boolean
  bigButtons: boolean
  highContrast: boolean
  /**
   * How the game takes the steering: `relative` (its keyboard's and gamepad's profiles: the plugin turns the game's
   * wheel toward the phone's angle) or `absolute` (a steering wheel's profile: the angle as it is).
   */
  gameSteering: 'relative' | 'absolute'
  /** The page in full screen, the phone's bars hidden (where the browser lets it). */
  fillScreen: boolean
  /** Degrees the drawn wheel turns at full lock (each way), as the truck's own does: 90 to 900. */
  drawnLock: number
  /** The pedals' sizes (owner 2026-11-19): each one's width, in percent of the screen's (15 to 40), and both's
   * height, in percent of the screen's (50 to 100). */
  throttleWidth: number
  brakeWidth: number
  pedalHeight: number
}

export const DEFAULTS: WheelSettings = {
  sensitivity: 1,
  range: 90,
  smoothing: 0.1,
  deadzone: 2,
  curve: 0.2,
  invert: false,
  throttleSensitivity: 1,
  throttleCurve: 0.3,
  brakeSensitivity: 1,
  brakeCurve: 0.4,
  pedalMode: 'touch',
  tiltRange: 25,
  tiltInvert: false,
  vibration: true,
  vibrationStrength: 2,
  leftHanded: false,
  bigButtons: false,
  highContrast: false,
  fillScreen: true,
  gameSteering: 'relative',
  drawnLock: 450,
  throttleWidth: 26,
  brakeWidth: 26,
  pedalHeight: 94,
}

type NumberKey = { [K in keyof WheelSettings]: WheelSettings[K] extends number ? K : never }[keyof WheelSettings]

/** Each number's least, most and step, for the settings panel and for reading what was saved. */
export const LIMITS: Record<NumberKey, { min: number; max: number; step: number }> = {
  sensitivity: { min: 0.5, max: 2, step: 0.05 },
  range: { min: 45, max: 180, step: 5 },
  smoothing: { min: 0, max: 0.9, step: 0.05 },
  deadzone: { min: 0, max: 10, step: 0.5 },
  curve: { min: 0, max: 1, step: 0.05 },
  throttleSensitivity: { min: 0.5, max: 2, step: 0.05 },
  throttleCurve: { min: 0, max: 1, step: 0.05 },
  brakeSensitivity: { min: 0.5, max: 2, step: 0.05 },
  brakeCurve: { min: 0, max: 1, step: 0.05 },
  tiltRange: { min: 10, max: 45, step: 1 },
  vibrationStrength: { min: 1, max: 3, step: 1 },
  drawnLock: { min: 90, max: 900, step: 15 },
  throttleWidth: { min: 15, max: 40, step: 1 },
  brakeWidth: { min: 15, max: 40, step: 1 },
  pedalHeight: { min: 50, max: 100, step: 2 },
}

const clamp = (value: number, min: number, max: number) => Math.min(max, Math.max(min, value))

/** Settings saved on the phone (JSON, or nothing), each read back within its limits; the rest the defaults. */
export function readSettings(saved: string | null): WheelSettings {
  let parsed: Record<string, unknown> = {}
  try {
    const value: unknown = saved ? JSON.parse(saved) : {}
    if (value && typeof value === 'object') parsed = value as Record<string, unknown>
  } catch {
    // Unreadable: the defaults.
  }
  const settings: WheelSettings = { ...DEFAULTS }
  for (const key of Object.keys(DEFAULTS) as (keyof WheelSettings)[]) {
    const value = parsed[key]
    const fallback = DEFAULTS[key]
    if (typeof fallback === 'number' && typeof value === 'number' && Number.isFinite(value)) {
      const limit = LIMITS[key as NumberKey]
      ;(settings as unknown as Record<string, unknown>)[key] = clamp(value, limit.min, limit.max)
    } else if (typeof fallback === 'boolean' && typeof value === 'boolean') {
      ;(settings as unknown as Record<string, unknown>)[key] = value
    } else if (key === 'pedalMode' && (value === 'touch' || value === 'tilt' || value === 'onoff')) {
      settings.pedalMode = value
    } else if (key === 'gameSteering' && (value === 'relative' || value === 'absolute')) {
      settings.gameSteering = value
    }
  }
  return settings
}

/** A response curve over 0..1: `k` 0 linear, 1 progressive (x³); always 0 at 0 and 1 at 1, and rising. */
export function curve(x: number, k: number): number {
  const at = clamp(x, 0, 1)
  return at ** (1 + 2 * clamp(k, 0, 1))
}

/** An angle in degrees wrapped to -180..180. */
export function wrap(degrees: number): number {
  const wrapped = ((((degrees + 180) % 360) + 360) % 360) - 180
  return wrapped === -180 ? 180 : wrapped
}

/**
 * The steering from the phone's turn away from its centre (degrees, counter-clockwise as the screen faces the
 * player, so to the left): -1 full left to 1 full right.
 */
export function shapeSteering(turned: number, settings: WheelSettings): number {
  const toRight = settings.invert ? turned : -turned
  const size = Math.abs(toRight)
  const dead = Math.min(settings.deadzone, settings.range - 1)
  if (size <= dead) return 0
  const x = clamp(((size - dead) / (settings.range - dead)) * settings.sensitivity, 0, 1)
  return Math.sign(toRight) * curve(x, settings.curve)
}

/** A pedal (0..1, how far it's pressed) by its sensitivity and curve. */
export function shapePedal(pressed: number, sensitivity: number, k: number): number {
  return curve(clamp(pressed, 0, 1) * sensitivity, k)
}

/**
 * How far a pedal is pressed by where it's touched: `fromTop` 0 at the pedal's top to 1 at its bottom. The top
 * tenth is all the way down; the very bottom, barely.
 */
export function touchToPedal(fromTop: number): number {
  return clamp((1 - fromTop) / 0.9, 0, 1)
}

/** Tilt mode: the throttle and brake from the phone's tilt away from its centre (degrees, forward positive). */
export function tiltToPedals(tilted: number, settings: WheelSettings): { throttle: number; brake: number } {
  const forward = settings.tiltInvert ? -tilted : tilted
  const dead = 3
  const span = Math.max(1, settings.tiltRange - dead)
  return {
    throttle: clamp((forward - dead) / span, 0, 1),
    brake: clamp((-forward - dead) / span, 0, 1),
  }
}

/**
 * One step of the steering's filter: from `previous` toward `next`, by `smoothing` (0 straight there) over `ms`
 * since the last step, alike at any rate the phone's sensor runs at.
 */
export function smooth(previous: number, next: number, smoothing: number, ms: number): number {
  const s = clamp(smoothing, 0, 0.95)
  if (s === 0) return next
  const keep = s ** (clamp(ms, 0, 200) / (1000 / 60))
  return next + (previous - next) * keep
}

/**
 * The values as the host reads them (crates/host wheel.rs): `w <steering> <throttle> <brake>`, and ` a` when the
 * game's steering is absolute (a wheel's profile; the plugin then gives the angle as it is, else it turns the game's
 * wheel toward it).
 */
export function encode(steering: number, throttle: number, brake: number, absolute = false): string {
  const at = (value: number, min: number) => clamp(Number.isFinite(value) ? value : 0, min, 1).toFixed(3)
  return `w ${at(steering, -1)} ${at(throttle, 0)} ${at(brake, 0)}${absolute ? ' a' : ''}`
}
