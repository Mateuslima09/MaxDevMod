import { describe, expect, it } from 'vitest'
import { Orientation } from './sensor'
import {
  DEFAULTS,
  curve,
  encode,
  readSettings,
  shapePedal,
  shapeSteering,
  smooth,
  tiltToPedals,
  touchToPedal,
  wrap,
} from './shape'

const linear = { ...DEFAULTS, deadzone: 0, curve: 0, sensitivity: 1, range: 90 }

describe('curve', () => {
  it('runs from 0 to 1 and only rises', () => {
    for (const k of [0, 0.3, 1]) {
      expect(curve(0, k)).toBe(0)
      expect(curve(1, k)).toBe(1)
      let last = -1
      for (let x = 0; x <= 1; x += 0.05) {
        expect(curve(x, k)).toBeGreaterThanOrEqual(last)
        last = curve(x, k)
      }
    }
  })
  it('is linear at 0 and gentler in the middle when progressive', () => {
    expect(curve(0.5, 0)).toBeCloseTo(0.5)
    expect(curve(0.5, 1)).toBeCloseTo(0.125)
    expect(curve(2, 0)).toBe(1)
    expect(curve(-1, 0)).toBe(0)
  })
})

describe('steering', () => {
  it('turning the phone left (counter-clockwise) steers left, and full lock at the range', () => {
    expect(shapeSteering(45, linear)).toBeCloseTo(-0.5)
    expect(shapeSteering(-45, linear)).toBeCloseTo(0.5)
    expect(shapeSteering(90, linear)).toBeCloseTo(-1)
    expect(shapeSteering(170, linear)).toBe(-1)
  })
  it('inverts', () => {
    expect(shapeSteering(45, { ...linear, invert: true })).toBeCloseTo(0.5)
  })
  it('sensitivity reaches full lock sooner', () => {
    expect(shapeSteering(-45, { ...linear, sensitivity: 2 })).toBeCloseTo(1)
    expect(shapeSteering(-45, { ...linear, sensitivity: 0.5 })).toBeCloseTo(0.25)
  })
  it('the range is the phone turn for full lock', () => {
    expect(shapeSteering(-45, { ...linear, range: 45 })).toBeCloseTo(1)
    expect(shapeSteering(-90, { ...linear, range: 180 })).toBeCloseTo(0.5)
  })
  it('the deadzone steers nothing, then starts from 0 (no jump)', () => {
    const dead = { ...linear, deadzone: 5 }
    expect(shapeSteering(4.9, dead)).toBe(0)
    expect(shapeSteering(-5, dead)).toBe(0)
    expect(Math.abs(shapeSteering(-5.5, dead))).toBeLessThan(0.01)
    expect(shapeSteering(-90, dead)).toBeCloseTo(1)
  })
  it('is progressive with a curve', () => {
    expect(shapeSteering(-45, { ...linear, curve: 1 })).toBeCloseTo(0.125)
  })
})

describe('pedals', () => {
  it('by the touch height: the top all the way, the bottom barely', () => {
    expect(touchToPedal(0)).toBe(1)
    expect(touchToPedal(0.1)).toBeCloseTo(1)
    expect(touchToPedal(1)).toBe(0)
    expect(touchToPedal(0.55)).toBeCloseTo(0.5)
  })
  it('sensitivity and curve', () => {
    expect(shapePedal(0.5, 1, 0)).toBeCloseTo(0.5)
    expect(shapePedal(0.5, 2, 0)).toBe(1)
    expect(shapePedal(0.5, 1, 1)).toBeCloseTo(0.125)
    expect(shapePedal(1, 0.5, 0)).toBeCloseTo(0.5)
  })
  it('by tilting: forward the throttle, back the brake, never both', () => {
    const settings = { ...DEFAULTS, tiltRange: 23 }
    expect(tiltToPedals(0, settings)).toEqual({ throttle: 0, brake: 0 })
    expect(tiltToPedals(2, settings)).toEqual({ throttle: 0, brake: 0 })
    expect(tiltToPedals(13, settings).throttle).toBeCloseTo(0.5)
    expect(tiltToPedals(13, settings).brake).toBe(0)
    expect(tiltToPedals(-40, settings)).toEqual({ throttle: 0, brake: 1 })
    expect(tiltToPedals(-13, { ...settings, tiltInvert: true }).throttle).toBeCloseTo(0.5)
  })
})

describe('smoothing', () => {
  it('none goes straight there', () => {
    expect(smooth(0, 1, 0, 16)).toBe(1)
  })
  it('more is slower, alike at any sensor rate', () => {
    const light = smooth(0, 1, 0.3, 1000 / 60)
    const heavy = smooth(0, 1, 0.8, 1000 / 60)
    expect(light).toBeCloseTo(0.7)
    expect(heavy).toBeLessThan(light)
    const twice = smooth(smooth(0, 1, 0.5, 1000 / 120), 1, 0.5, 1000 / 120)
    expect(twice).toBeCloseTo(smooth(0, 1, 0.5, 1000 / 60))
  })
})

describe('settings', () => {
  it('reads back what was saved, within its limits, the rest the defaults', () => {
    const read = readSettings(
      JSON.stringify({ sensitivity: 9, range: 120, invert: true, pedalMode: 'tilt', bigButtons: 'yes', extra: 1 }),
    )
    expect(read.sensitivity).toBe(2)
    expect(read.range).toBe(120)
    expect(read.invert).toBe(true)
    expect(read.pedalMode).toBe('tilt')
    expect(read.bigButtons).toBe(false)
    expect(read.smoothing).toBe(DEFAULTS.smoothing)
    expect('extra' in read).toBe(false)
  })
  it('the defaults when nothing or something unreadable was saved', () => {
    expect(readSettings(null)).toEqual(DEFAULTS)
    expect(readSettings('{nope')).toEqual(DEFAULTS)
    expect(readSettings(JSON.stringify({ pedalMode: 'jump' })).pedalMode).toBe('touch')
    expect(readSettings(JSON.stringify({ gameSteering: 'absolute' })).gameSteering).toBe('absolute')
    expect(readSettings(JSON.stringify({ gameSteering: 'x' })).gameSteering).toBe('relative')
  })
})

describe('the message', () => {
  it('is what the host reads, clamped', () => {
    expect(encode(-0.25, 0.8, 0)).toBe('w -0.250 0.800 0.000')
    expect(encode(3, -1, Number.NaN)).toBe('w 1.000 0.000 0.000')
    expect(encode(0.5, 0, 0, true)).toBe('w 0.500 0.000 0.000 a')
  })
})

describe('wrap', () => {
  it('keeps angles in -180..180', () => {
    expect(wrap(190)).toBe(-170)
    expect(wrap(-190)).toBe(170)
    expect(wrap(360)).toBe(0)
    expect(wrap(180)).toBe(180)
  })
})

describe('the phone turned like a wheel', () => {
  /** Gravity's pull as a phone held upright, turned `turn` degrees counter-clockwise, would measure it. */
  const gravity = (turn: number) => {
    const r = (turn * Math.PI) / 180
    return [9.81 * Math.sin(r), 9.81 * Math.cos(r), 0] as const
  }

  it('follows gravity alone without a gyroscope', () => {
    const o = new Orientation()
    for (let t = 0; t < 60; t++) o.update(...gravity(30), null, t * 16)
    expect(o.turn).toBeCloseTo(30, 0)
  })

  it('follows the gyroscope at once, and finds its sense when the phone counts it the other way', () => {
    for (const sense of [1, -1]) {
      const o = new Orientation()
      let turn = 0
      o.update(...gravity(turn), 0, 0)
      // Turning at 90°/s, back and forth, for a while.
      for (let step = 1; step < 400; step++) {
        const rate = Math.floor(step / 60) % 2 === 0 ? 90 : -90
        turn += (rate * 16) / 1000
        o.update(...gravity(turn), rate * sense, step * 16)
      }
      expect(Math.abs(wrap((o.turn ?? 0) - turn))).toBeLessThan(3)
    }
  })

  it('the tilt: the top edge away is forward', () => {
    const o = new Orientation()
    o.update(0, 9.81 * Math.cos(0.3), 9.81 * Math.sin(0.3), 0, 0)
    expect(o.tilt).toBeCloseTo((0.3 * 180) / Math.PI, 0)
  })
})
